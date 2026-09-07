//! Uppercase tags: the three things a capitalized name resolves to, a
//! component, a template (Rust or `.bsx`) or a resource declaration, plus the
//! props store a `.bsx` template's attributes materialize into.

use super::binding::*;
use super::directives::*;
use super::element::*;
use super::entity_refs::*;
use super::literal::*;
#[cfg(feature = "bevy_async")]
use super::remote::*;
use super::spread::*;
use crate::bsx::reflect::*;
use crate::prelude::*;
use bevy::ecs::template::TemplateContext;

/// Build a capitalized tag: a component (reflect-patched) or a template (built
/// with input props), incl `<path::to::X>` BSX templates.
pub(super) fn build_uppercase(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	// a custom-tag handler (eg `<Rule>`) resolves the whole tag before the type
	// registry: it reads the raw attributes and mutates the world, producing no
	// entity content. Core registers none; a higher layer installs them.
	if let Some(handler) = cx.entity.world_scope(|world| {
		world.get_resource::<BsxTagResolvers>()?.get(&el.tag)
	}) {
		return handler(el, cx.entity);
	}

	// `<Template src="..">` is the remote-template front-end: register a pending
	// fetch into the root's pending set, awaited by `Ready`. Remote
	// resolution needs the async runtime, so it is `bevy_async`-gated.
	if el.tag == "Template" {
		if let Some(_src) = string_attr(el, "src") {
			#[cfg(feature = "bevy_async")]
			register_remote_template(SmolStr::from(_src.as_str()), cx)?;
		}
		apply_common_directives(el, refs, cx)?;
		return Ok(());
	}

	// a `<path::to::X>` BSX template resolves from the registry first.
	if let Some(def) = registry.get(&el.tag) {
		// a remote schema resolves asynchronously, deferring `Ready`.
		if let Some(_url) = def.remote_schema.clone() {
			#[cfg(feature = "bevy_async")]
			register_remote_schema(SmolStr::from(el.tag.as_str()), _url, cx)?;
		}
		// verify props against the template's inline `bx:schema`, if it declared one.
		if let Some(schema) = def.schema.clone() {
			verify_props_against(el, &el.tag, &schema, cx)?;
		}
		// pre-resolve `$name` refs (incl `@entity:ref::` selectors) for the props
		// store and spreads.
		let entity_refs = resolve_entity_refs(el, refs, cx);
		// materialize the props store before the body builds, so the body's
		// `DocumentPath::Props` bindings link against it on insert.
		apply_props_store(el, cx.entity, &entity_refs)?;
		let nested = BsxTemplate::new(def.nodes.clone(), registry.clone());
		// build the template's subtree into this entity, carrying its slot targets.
		cx.entity.build_template(&nested)?;
		apply_common_directives(el, refs, cx)?;
		apply_spreads(el, cx.entity, &entity_refs)?;
		// caller content becomes slot children on this entity.
		build_slot_children(el, registry, refs, cx)?;
		return Ok(());
	}

	let app_registry = cx
		.entity
		.world_scope(|world| world.get_resource::<AppTypeRegistry>().cloned())
		.ok_or_else(|| {
			bevyhow!(
				"resolving the `<{}>` tag requires an `AppTypeRegistry`",
				el.tag
			)
		})?;
	// pre-resolve every `$name` in this tag's attributes to a real (forward-mapped)
	// entity, so patch building has a plain name->entity lookup.
	let entity_refs = resolve_entity_refs(el, refs, cx);
	let registration_kind = {
		let registry = app_registry.read();
		// resolve by base name so a generic tag (eg `<Repeat>` -> `Repeat<()>`)
		// matches its sole instantiation, like a `{Repeat}` spread does.
		registration_by_name(&registry, &el.tag)
			.map(|registration| {
				let kind = if registration.data::<ReflectTemplate>().is_some() {
					UppercaseKind::Template
				} else if registration
					.data::<bevy::ecs::reflect::ReflectResource>()
					.is_some()
				{
					UppercaseKind::Resource
				} else {
					UppercaseKind::Component
				};
				(kind, registration.type_info())
			})
			.map(|(kind, info)| {
				(kind, build_patch(el, info, &registry, &entity_refs))
			})
	};

	let Some((kind, patch)) = registration_kind else {
		// a known featured-out tag (eg `<LiveReloadScript/>` with `client_io`
		// compiled out) resolves to nothing at all, children included.
		if is_allowed_unregistered(cx, &el.tag) {
			return Ok(());
		}
		return build_unregistered(el, registry, refs, &entity_refs, cx);
	};
	let patch = patch?;

	match kind {
		UppercaseKind::Template => {
			// verify the props against the template's schema before building, so a
			// missing required field or a type mismatch is a graceful error.
			verify_props(el, &el.tag, &app_registry, cx)?;
			// build the registered template into this entity, then route caller content.
			build_template_by_name(&app_registry, &el.tag, patch.as_ref(), cx)?;
			apply_common_directives(el, refs, cx)?;
			apply_spreads(el, cx.entity, &entity_refs)?;
			build_slot_children(el, registry, refs, cx)?;
		}
		UppercaseKind::Resource => {
			// a resource declaration: patch the live resource, no entity content.
			apply_resource_tag(el, patch.as_ref(), &app_registry, cx)?;
		}
		UppercaseKind::Component => {
			// a tag-position literal (`<Name("x")/>`, `<Log::Message("hi")/>`)
			// builds the component from the literal, exactly as the `{..}` spread
			// position does, instead of patching over `Default::default()`.
			match &el.tag_literal {
				Some(named) => apply_spread_named(
					named,
					cx.entity,
					&app_registry,
					&entity_refs,
				)?,
				// a bare/attribute component: reflect-patch over default and insert.
				None => {
					insert_component(cx.entity, patch.as_ref(), &app_registry)?
				}
			}
			// a `<MyComponent value=@doc:path>` binding syncs the source field with
			// the component field, both ways, via a reflect-field binding.
			apply_component_field_bindings(el, cx.entity)?;
			apply_common_directives(el, refs, cx)?;
			apply_spreads(el, cx.entity, &entity_refs)?;
			build_children(el, registry, refs, cx)?;
		}
	}
	Ok(())
}

/// Build a tag nothing is registered under: warn, record the name in an
/// [`UnregisteredTag`] marker, and build the node's directives, spreads and
/// children onto the now-inert entity.
///
/// Structure is universal, so the entity exists in every binary and only its
/// behavior is missing: a `bx:ref` into the region still names a real entity,
/// and a lean build's tree has the same shape as a full one's. Attributes are
/// dropped, since they are props of a type this binary does not have; a spread
/// is not, since it names its own types and skips the ones it cannot resolve.
///
/// A subtree whose self-or-ancestor [`RequireFeatures`] is unmet has declared
/// its own inertness, so its tags build quietly at `debug!`; anywhere else the
/// tag warns. A typo is caught by `beet check`, which registers everything and
/// elevates every marker it finds to an error.
fn build_unregistered(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	entity_refs: &HashMap<SmolStr, Entity>,
	cx: &mut TemplateContext,
) -> Result<()> {
	cx.entity.insert(UnregisteredTag::new(el.tag.as_str()));
	apply_common_directives(el, refs, cx)?;
	// spreads before the log, so a host declaring its own `RequireFeatures`
	// quiets its own tag as well as its subtree's.
	apply_spreads(el, cx.entity, entity_refs)?;
	if inertness_declared(cx.entity) {
		debug!(
			"tag `<{}>` is not registered in this binary, building it inert (declared by `RequireFeatures`)",
			el.tag
		);
	} else {
		warn!(
			"no component, resource or template registered for tag `<{}>`, building it inert",
			el.tag
		);
	}
	build_children(el, registry, refs, cx)
}

/// Whether a self-or-ancestor [`RequireFeatures`] is unmet in this binary, ie
/// the subtree has declared that its behavior is feature-dependent and the
/// features are absent, making an unresolvable tag or spread the expected
/// state rather than a surprise. Ancestry exists mid-build: a child spawns
/// with its `ChildOf`, and a parent's spreads apply before its children build.
pub(super) fn inertness_declared(entity: &mut EntityWorldMut) -> bool {
	let id = entity.id();
	entity.world_scope(|world| {
		world.with_state::<(
			AncestorQuery<&RequireFeatures>,
			Query<&CrateRegistration>,
		), _>(|(requires, registrations)| {
			requires
				.get_ancestors(id)
				.iter()
				.any(|require| !require.failures(&registrations).is_empty())
		})
	})
}

/// How an uppercase tag's type registration resolves.
pub(super) enum UppercaseKind {
	/// A `#[template]` type ([`ReflectTemplate`]).
	Template,
	/// A `#[reflect(Resource)]` type: a resource declaration.
	Resource,
	/// A plain reflected component.
	Component,
}

/// Insert a reflect-field binding for every binding-valued attribute on a
/// component tag (`<MyComponent value=@doc:path>`), so the source field syncs
/// with the component field both ways:
/// `source <-> Value <-> MyComponent.field`.
///
/// The first such attribute owns the entity's binding: the source components
/// via [`apply_binding`] plus a [`ReflectFieldRef`] sink naming the tag's
/// component and field. An `@comp` source is rejected, an entity carries at
/// most one [`ReflectFieldRef`].
fn apply_component_field_bindings(
	el: &BsxElement,
	entity: &mut EntityWorldMut,
) -> Result<()> {
	for attr in &el.attributes {
		if is_directive(&attr.key) || attr.key.is_empty() {
			continue;
		}
		match &attr.value {
			AttrValue::Expr(ValueExpr::Binding(binding)) => {
				if binding.source == BindingSource::Comp {
					bevybail!(
						"`{}={}` cannot bind a component field to another component field",
						attr.key,
						"@comp:.."
					);
				}
				apply_binding(binding, entity, BindingTarget::This)?;
			}
			_ => continue,
		}
		// the `Value`<->reflect bridge needs `serde_json` (the `json` feature); an
		// embedded build without it keeps the source binding but loses the
		// bidirectional reflect-field write (Risk: documented, acceptable).
		#[cfg(feature = "json")]
		{
			let component = el.tag.rsplit("::").next().unwrap_or(&el.tag);
			entity.insert(ReflectFieldRef::new(component, attr.key.as_str()));
		}
		// one binding per entity: the first binding-valued attribute owns it.
		break;
	}
	Ok(())
}

/// Materialize a `.bsx` registry tag's prop attributes as a reactive props
/// store on the template's entity, so the body binds to them via
/// [`DocumentPath::Props`] (while [`DocumentPath::Ancestor`] skips the store).
///
/// Literal props seed the store's [`Document`]. Each binding-valued prop
/// (`title=@doc:field`, `@res`, `@comp`, `@prop`)
/// additionally spawns a binding entity chaining
/// `source -> Value <-> props.title`, which the document sync fans out to the
/// body. The binding entity relates via [`AttributeOf`] rather than as a
/// child, so it never renders and despawns with the template entity.
fn apply_props_store(
	el: &BsxElement,
	entity: &mut EntityWorldMut,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let store = entity.id();
	let mut props = props_value(el);
	let mut bindings = Vec::new();
	for attr in &el.attributes {
		if is_directive(&attr.key) || attr.key.is_empty() {
			continue;
		}
		let binding = match &attr.value {
			AttrValue::Expr(ValueExpr::Binding(binding)) => binding.clone(),
			_ => continue,
		};
		// pre-seed the bound key (`=init` or null) so a freshly added body
		// Value never racily seeds it via write-back before the source lands.
		let seed = binding
			.init
			.as_ref()
			.map(literal_to_value)
			.transpose()?
			.unwrap_or_default();
		props
			.as_map_mut()?
			.insert(SmolStr::from(attr.key.as_str()), seed);
		bindings.push((
			prop_binding_source(&binding, store, entity_refs),
			FieldRef::new(attr.key.as_str())
				.with_document(DocumentPath::Entity(store)),
		));
	}
	entity.insert((Document::new(props), PropsDocument));
	entity.world_scope(|world| {
		for (source, sink) in bindings {
			let mut binding_entity =
				world.spawn((AttributeOf::new(store), Value::default(), sink));
			match source {
				PropBindingSource::Field(source) => {
					binding_entity.insert(source);
				}
				#[cfg(feature = "json")]
				PropBindingSource::Resource(source) => {
					binding_entity.insert(source);
				}
				#[cfg(feature = "json")]
				PropBindingSource::Component(source) => {
					binding_entity.insert(source);
				}
				#[cfg(not(feature = "json"))]
				PropBindingSource::Seed => {}
			}
		}
	});
	Ok(())
}

/// The source component of a props binding entity, mirroring the tag-site
/// binding source into the entity's [`Value`].
enum PropBindingSource {
	/// `@doc`/`@prop`: a one-way document mirror.
	Field(SourceFieldRef),
	/// `@res`: a bidirectional resource field sync.
	#[cfg(feature = "json")]
	Resource(ResourceFieldRef),
	/// `@comp`: a bidirectional component field sync.
	#[cfg(feature = "json")]
	Component(ReflectFieldRef),
	/// `@res`/`@comp` without the `json` reflect bridge: the seed only.
	#[cfg(not(feature = "json"))]
	Seed,
}

/// Build the source component for a binding-valued prop. The document-sourced
/// kinds resolve from the `store` (the tag site), since the binding entity
/// itself is outside the `ChildOf` hierarchy; a selector-less `@comp` also
/// targets the store, ie a component co-located on the template's entity.
fn prop_binding_source(
	binding: &BindingExpr,
	store: Entity,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> PropBindingSource {
	#[cfg(not(feature = "json"))]
	let _ = entity_refs;
	match binding.source {
		BindingSource::Doc => SourceFieldRef::new(binding.field_path.clone())
			.with_subject(store)
			.xmap(PropBindingSource::Field),
		BindingSource::Prop => SourceFieldRef::new(binding.field_path.clone())
			.with_document(DocumentPath::Props)
			.with_subject(store)
			.xmap(PropBindingSource::Field),
		#[cfg(feature = "json")]
		BindingSource::Res => ResourceFieldRef::new(
			binding.type_path.clone().unwrap_or_default(),
			binding.field_path.to_string(),
		)
		.xmap(PropBindingSource::Resource),
		#[cfg(feature = "json")]
		BindingSource::Comp => {
			let target = match &binding.selector {
				Some(name) => map_selector_target(name, entity_refs),
				None => BindingTarget::Entity(store),
			};
			ReflectFieldRef::new(
				binding.type_path.clone().unwrap_or_default(),
				binding.field_path.to_string(),
			)
			.with_target(target)
			.xmap(PropBindingSource::Component)
		}
		#[cfg(not(feature = "json"))]
		BindingSource::Res | BindingSource::Comp => PropBindingSource::Seed,
	}
}

/// Lower a resource declaration tag (`<PackageConfig title=".."/>`): the
/// literal attrs patch the named fields of the live resource, the rest keep
/// their current values. An absent resource inserts over the type's default
/// (which needs `#[reflect(Default)]` or a complete patch). The element
/// produces no entity content, like a directive-only node.
fn apply_resource_tag(
	el: &BsxElement,
	patch: &dyn bevy::reflect::PartialReflect,
	app_registry: &AppTypeRegistry,
	cx: &mut TemplateContext,
) -> Result<()> {
	if !el.children.is_empty() {
		bevybail!(
			"`<{}>` declares a resource and cannot have children",
			el.tag
		);
	}
	// a resource declaration is a one-shot patch, not a sync target.
	if let Some(attr) = el.attributes.iter().find(|attr| {
		matches!(&attr.value, AttrValue::Expr(ValueExpr::Binding(_)))
	}) {
		bevybail!(
			"`<{} {}=@..>`: an `@` binding cannot declare a resource field, use a literal",
			el.tag,
			attr.key
		);
	}
	cx.entity.world_scope(|world| -> Result<()> {
		write_resource_patch(world, app_registry, &el.tag, patch)
	})
}

/// Marks the backing entity of a resource a markup scene declared, recording the
/// template build root that created it, so scene teardown (`despawn_scene`)
/// removes the resource *with that scene* and a rebuild reinserts it fresh from
/// markup rather than patching stale live state.
///
/// The root is carried rather than implied so teardown is scoped: with two scenes
/// loaded, despawning one must not take the other's resources with it. A
/// teardown removes only the resources whose root is among the roots it is
/// despawning, and treats a resource whose root has already gone as orphaned (it
/// goes too, since nothing can rebuild it).
///
/// Only set when the markup *creates* the resource inside a template build; a
/// resource that pre-existed (a plugin default) keeps its own lifetime and the
/// markup only patches it. Edge case: two scenes declaring the same resource
/// share one backing entity (the first creates and owns it, the second only
/// patches), so tearing down the creating scene removes the resource for both.
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct SceneResource {
	/// The template build root whose scene created this resource.
	pub root: Entity,
}

/// Write a reflect patch to a [`ReflectResource`]-backed type: patch the live
/// resource (missing fields keep their values) or, when absent, spawn it over
/// the type's default. Resources are entity-backed, so this writes through the
/// resource's implied [`ReflectComponent`]. `tag` only names the type in errors
/// (an uppercase tag's name or a spread name). Shared by the resource-tag path
/// ([`apply_resource_tag`]) and the spread path ([`apply_spread_named`]).
pub(super) fn write_resource_patch(
	world: &mut World,
	app_registry: &AppTypeRegistry,
	tag: &str,
	patch: &dyn bevy::reflect::PartialReflect,
) -> Result<()> {
	use bevy::ecs::reflect::ReflectComponent;
	let registry = app_registry.read();
	let type_info = patch
		.get_represented_type_info()
		.ok_or_else(|| bevyhow!("resource patch has no represented type"))?;
	let registration = registry.get(type_info.type_id()).ok_or_else(|| {
		bevyhow!("type `{}` is not registered", type_info.type_path())
	})?;
	// resources are entity-backed: write through the implied ReflectComponent.
	let reflect_component = registration
		.data::<ReflectComponent>()
		.expect("ReflectComponent is depended on by ReflectResource");
	let component_id = reflect_component.register_component(world);
	match world.resource_entities().get(component_id) {
		// patch the live resource: missing fields keep their values.
		Some(resource_entity) => reflect_component
			.apply(&mut world.entity_mut(resource_entity), patch),
		// absent: insert the patch over the type's default.
		None => {
			use bevy::ecs::reflect::ReflectFromWorld;
			use bevy::reflect::ReflectFromReflect;
			use bevy::reflect::std_traits::ReflectDefault;
			// ReflectComponent::insert panics on unconstructible types,
			// so check before reaching it
			let constructible = registration.data::<ReflectDefault>().is_some()
				|| registration.data::<ReflectFromWorld>().is_some()
				|| registration.data::<ReflectFromReflect>().is_some_and(
					|from_reflect| from_reflect.from_reflect(patch).is_some(),
				);
			if !constructible {
				bevybail!(
					"`{}`: the resource is not in the world and `{}` cannot be constructed from the patch, add `#[reflect(Default)]` or insert the resource first",
					tag,
					type_info.type_path()
				);
			}
			// created by the markup: tie the backing entity to the build root (when
			// inside a template build) so that scene's teardown, and only that
			// scene's, removes the resource with it.
			let scene_root =
				world.get_resource::<TemplateBuildRoot>().map(|root| **root);
			let resource_entity = world.spawn_empty().id();
			reflect_component.insert(
				&mut world.entity_mut(resource_entity),
				patch,
				&registry,
			);
			if let Some(root) = scene_root {
				world
					.entity_mut(resource_entity)
					.insert(SceneResource { root });
			}
		}
	}
	Ok(())
}
