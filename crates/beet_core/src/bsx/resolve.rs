//! AST-to-world resolution: build a [`BsxNode`] tree into an entity through the
//! substrate, producing trees identical to what `rsx!` lowers to.
//!
//! [`BsxTemplate`] is the [`Template`] every BSX front-end produces. Its build
//! walks the syntax tree into `cx.entity`:
//!
//! - a lowercase tag becomes an [`Element`] with attribute child entities;
//! - an uppercase tag resolves by name to a component (reflect-patched), a
//!   resource declaration (`<PackageConfig title=".."/>` patches the live
//!   resource, no entity content), or a template (built with input props),
//!   incl `<path::to::X>` BSX templates, whose props materialize as a reactive
//!   `(Document, PropsDocument)` store;
//! - a bare spread inserts its resolved components/templates onto the entity;
//! - an `@` binding lowers to its source's sync components: `@doc`/`@prop` to a
//!   [`FieldRef`], `@res` to a `(Value, ResourceFieldRef)`, `@comp` to a
//!   `(Value, ReflectFieldRef)` targeting the current entity, the element (in
//!   attribute position), or an `@entity:Name::` named entity;
//! - the reserved selector names ([`ReservedRef`]) target well-known entities
//!   instead of `bx:ref` names (which may not shadow them): `BuildRoot` and
//!   `SnippetRoot` resolve at build time, `PageRoot` and `Router` lazily
//!   in the sync pass via [`BindingTarget::Reserved`];
//! - a `$`reference resolves to a `bx:ref`-named entity through the one entity
//!   model;
//! - `bx:scope`/`bx:for`+`bx:key`/`<Slot>`/`bx:slot`/`bx:click` lower to their
//!   document-system and slot-marker components.
//!
//! Entity references resolve through `cx.entity_references` with a two-pass walk
//! (collect `bx:ref` names, then resolve `$name`), so `$name` may point forward.

use super::ast::*;
use super::events::*;
use super::reflect::*;
use super::registry::*;
use super::schema::props_value;
use crate::prelude::*;
use bevy::ecs::template::SceneEntityReference;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::reflect::TypeRegistry;

/// A parsed BSX tree as a build-subtree [`Template`].
///
/// Holds its root nodes and the BSX-template registry snapshot, so resolving a
/// `<path::to::X>` tag needs no world lookup mid-build. Built once into the
/// caller's entity by `spawn_template`/`insert_template`.
#[derive(Clone)]
pub struct BsxTemplate {
	/// The root nodes to build into the calling entity.
	pub nodes: Vec<BsxNode>,
	/// A snapshot of the BSX-template registry for `<path::to::X>` resolution.
	pub registry: BsxTemplateRegistry,
	/// When `true`, the calling entity is a container and every root node spawns
	/// as its own child (the parse-a-document convention, matching the HTML
	/// parser). When `false`, a single root builds into the entity (the nested
	/// `<path::to::X>` body convention, so it composes onto the caller).
	pub as_container: bool,
}

subtree_template!(BsxTemplate);

impl BsxTemplate {
	/// A nested-template body: a single root builds into the calling entity.
	pub fn new(nodes: Vec<BsxNode>, registry: BsxTemplateRegistry) -> Self {
		Self {
			nodes,
			registry,
			as_container: false,
		}
	}

	/// A parsed document: the calling entity is a container, roots become children.
	pub fn container(
		nodes: Vec<BsxNode>,
		registry: BsxTemplateRegistry,
	) -> Self {
		Self {
			nodes,
			registry,
			as_container: true,
		}
	}
}

impl Template for BsxTemplate {
	type Output = ();
	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		// pass 1: collect every `bx:ref` name -> a pinned reference id, so a `$name`
		// forward reference resolves to the same placeholder entity.
		let mut refs = RefBindings::default();
		collect_refs(&self.nodes, &mut refs)?;
		// expose this build's root for `@entity:SnippetRoot::`, restoring any outer
		// snippet root so nested registry-template builds nest correctly.
		let root = cx.entity.id();
		// SAFETY: only used to swap the snippet-root resource, no flush.
		let world = unsafe { cx.entity.world_mut() };
		let previous = world.remove_resource::<SnippetBuildRoot>();
		world.insert_resource(SnippetBuildRoot(root));
		let result = if self.as_container {
			// every root node spawns as a child of the container entity.
			self.nodes.iter().try_for_each(|node| {
				spawn_child(node, root, &self.registry, &refs, cx).map(|_| ())
			})
		} else {
			build_root_nodes(&self.nodes, &self.registry, &refs, cx)
		};
		// SAFETY: only used to swap the snippet-root resource, no flush.
		let world = unsafe { cx.entity.world_mut() };
		match previous {
			Some(previous) => world.insert_resource(previous),
			None => {
				world.remove_resource::<SnippetBuildRoot>();
			}
		}
		result
	}
	fn clone_template(&self) -> Self { self.clone() }
}

/// The root entity of the innermost [`BsxTemplate`] build (the parsed document
/// container or a registry `<path::to::X>` body): the `SnippetRoot` reserved
/// target. Set for the duration of a build, mirroring [`TemplateBuildRoot`].
#[derive(Debug, Clone, Copy, Resource)]
struct SnippetBuildRoot(Entity);

/// Names declared by a `bx:ref` anywhere in the tree, each pinned to a stable
/// [`SceneEntityReference`] so a forward `$name` resolves identically.
#[derive(Default)]
struct RefBindings {
	names: HashMap<SmolStr, SceneEntityReference>,
}

impl RefBindings {
	/// The pinned reference for `name`, allocating a stable one on first use.
	fn reference(&mut self, name: &str) -> SceneEntityReference {
		let next = self.names.len();
		*self.names.entry(name.into()).or_insert_with(|| {
			SceneEntityReference::new(("bsx_ref", 0, 0), next)
		})
	}

	/// The pinned reference for `name`, if declared by a `bx:ref`.
	fn get(&self, name: &str) -> Option<SceneEntityReference> {
		self.names.get(name).copied()
	}
}

/// Walk the tree collecting every `bx:ref` name into stable references,
/// erroring on a name that shadows a reserved selector ([`ReservedRef`]).
fn collect_refs(nodes: &[BsxNode], refs: &mut RefBindings) -> Result<()> {
	for node in nodes {
		if let BsxNode::Element(el) = node {
			for attr in &el.attributes {
				if attr.key == "bx:ref" {
					if let AttrValue::Str(name) = &attr.value {
						if ReservedRef::parse(name).is_some() {
							bevybail!(
								"`bx:ref=\"{name}\"` shadows a reserved ref name, reserved: {:?}",
								ReservedRef::NAMES
							);
						}
						refs.reference(name);
					}
				}
			}
			collect_refs(&el.children, refs)?;
		}
	}
	Ok(())
}

/// The reserved `@entity:Name::` selector names, targeting well-known entities
/// instead of user `bx:ref` names. Declaring one via `bx:ref` is a build error
/// (see [`collect_refs`]), so a reserved selector is never ambiguous.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedRef {
	/// The outermost root of the current `spawn_template` build, resolved at
	/// build time from [`TemplateBuildRoot`].
	BuildRoot,
	/// The root of the innermost BSX template build (the parsed document or a
	/// registry template body), resolved at build time from [`SnippetBuildRoot`].
	SnippetRoot,
	/// The nearest self-or-ancestor entity carrying a `PageRoot` component,
	/// resolved lazily each sync pass: the render tree may not exist or be
	/// attached at build time (layouts build detached, per request).
	PageRoot,
	/// The nearest self-or-ancestor entity carrying a `Router` component,
	/// resolved lazily each sync pass.
	Router,
}

impl ReservedRef {
	/// Every reserved selector name.
	pub const NAMES: &[&str] =
		&["BuildRoot", "SnippetRoot", "PageRoot", "Router"];

	/// Classify a selector name, `None` for a user `bx:ref` name.
	pub fn parse(name: &str) -> Option<Self> {
		match name {
			"BuildRoot" => Some(Self::BuildRoot),
			"SnippetRoot" => Some(Self::SnippetRoot),
			"PageRoot" => Some(Self::PageRoot),
			"Router" => Some(Self::Router),
			_ => None,
		}
	}

	/// Resolve a build-time reserved name, `None` for the lazy names
	/// ([`Self::PageRoot`]/[`Self::Router`]), which resolve in the binding
	/// sync instead ([`BindingTarget::Reserved`]).
	fn build_time_entity(
		self,
		world: &World,
		fallback: Entity,
	) -> Option<Entity> {
		match self {
			Self::BuildRoot => {
				Some(TemplateBuildRoot::resolve(world, fallback))
			}
			Self::SnippetRoot => world
				.get_resource::<SnippetBuildRoot>()
				.map(|root| root.0)
				.unwrap_or(fallback)
				.xmap(Some),
			Self::PageRoot | Self::Router => None,
		}
	}

	/// The selector's [`BindingTarget`]: a build-time entity, or the lazy
	/// reserved `name` deferred to the sync pass.
	fn target(self, name: &SmolStr, cx: &mut TemplateContext) -> BindingTarget {
		let fallback = cx.entity.id();
		cx.entity
			.world_scope(|world| self.build_time_entity(world, fallback))
			.map(BindingTarget::Entity)
			.unwrap_or_else(|| BindingTarget::Reserved(name.clone()))
	}
}

/// Build the root nodes into the context entity. The first node builds into the
/// root; the rest spawn as children, matching the `rsx!` fragment lowering.
fn build_root_nodes(
	nodes: &[BsxNode],
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	match nodes {
		[] => Ok(()),
		[single] => build_node_into(single, registry, refs, cx),
		many => {
			// multiple roots: each spawns its own child entity under the root.
			let root = cx.entity.id();
			for node in many {
				spawn_child(node, root, registry, refs, cx)?;
			}
			Ok(())
		}
	}
}

/// Build a single node directly into `cx.entity` (the root case).
fn build_node_into(
	node: &BsxNode,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	match node {
		BsxNode::Element(el) => build_element(el, registry, refs, cx),
		_ => apply_leaf(node, refs, cx),
	}
}

/// Spawn `node` as a child of `parent`, returning the spawned entity.
fn spawn_child(
	node: &BsxNode,
	parent: Entity,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<Entity> {
	// a `bx:ref`-named element reuses its pinned reference entity (which a forward
	// `$name` may already have spawned as a placeholder), so the reference and the
	// built node are the same entity.
	let child = match node_ref_name(node).and_then(|name| refs.get(name)) {
		Some(reference) => {
			// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
			let world = unsafe { cx.entity.world_mut() };
			let child = cx.entity_references.get(reference, world);
			world.entity_mut(child).insert(ChildOf(parent));
			child
		}
		None => {
			// SAFETY: only used to spawn a child entity.
			let world = unsafe { cx.entity.world_mut() };
			world.spawn(ChildOf(parent)).id()
		}
	};
	build_node_at(node, child, registry, refs, cx)?;
	Ok(child)
}

/// The `bx:ref="name"` declared by an element node, if any.
fn node_ref_name(node: &BsxNode) -> Option<&str> {
	let BsxNode::Element(el) = node else {
		return None;
	};
	el.attributes.iter().find_map(|attr| {
		if attr.key != "bx:ref" {
			return None;
		}
		match &attr.value {
			AttrValue::Str(name) => Some(name.as_str()),
			_ => None,
		}
	})
}

/// Build `node` onto the already-spawned `entity`.
fn build_node_at(
	node: &BsxNode,
	entity: Entity,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	// SAFETY: scope a build into the target entity, sharing the reference map.
	let world = unsafe { cx.entity.world_mut() };
	let mut entity_mut = world.entity_mut(entity);
	let mut scoped =
		TemplateContext::new(&mut entity_mut, cx.entity_references);
	match node {
		BsxNode::Element(el) => build_element(el, registry, refs, &mut scoped),
		_ => apply_leaf(node, refs, &mut scoped),
	}
}

/// Apply a text/expr/comment/doctype leaf onto `cx.entity`.
fn apply_leaf(
	node: &BsxNode,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	match node {
		BsxNode::Text(text) => {
			cx.entity.insert(Value::Str(text.into()));
		}
		BsxNode::Expr(expr) => {
			// text position: an `@comp` binds this entity unless `$ref` retargets.
			let comp_target = match expr {
				ValueExpr::Binding(binding) => match &binding.selector {
					Some(name) => selector_target(name, refs, cx),
					None => BindingTarget::This,
				},
				_ => BindingTarget::This,
			};
			apply_value_expr(expr, cx.entity, comp_target)?;
		}
		BsxNode::Comment(content) => {
			cx.entity.insert(Comment::new(content.clone()));
		}
		BsxNode::Doctype(value) => {
			cx.entity.insert(Doctype::new(value.clone()));
		}
		BsxNode::Element(_) => unreachable!("handled before apply_leaf"),
	}
	Ok(())
}

/// Resolve a `$name` to its real, forward-mapped entity through the one entity
/// model, spawning the pinned placeholder on first use.
fn resolve_ref(
	name: &str,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Entity {
	let reference = refs.get(name).unwrap_or_else(|| stable_reference(name));
	// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
	let world = unsafe { cx.entity.world_mut() };
	cx.entity_references.get(reference, world)
}

/// Lower a text/attribute-position value expression onto `entity`: a literal
/// becomes a [`Value`], an `@` binding its binding components
/// ([`apply_binding`], with `comp_target` naming the entity an `@comp` binds),
/// a `$`reference is rejected (an entity reference is not a text value).
fn apply_value_expr(
	expr: &ValueExpr,
	entity: &mut EntityWorldMut,
	comp_target: BindingTarget,
) -> Result<()> {
	match expr {
		ValueExpr::Literal(literal) => {
			entity.insert(literal_to_value(literal)?);
		}
		ValueExpr::Binding(binding) => {
			apply_binding(binding, entity, comp_target)?;
		}
		ValueExpr::EntityRef(_) => {
			bevybail!(
				"`$name` entity references are not valid in text position"
			)
		}
	}
	Ok(())
}

/// Lower an `@` binding's components onto the value-bearing `entity`.
///
/// `comp_target` is the entity an `@comp` binding's component lives on: the
/// element in attribute position, the binding entity itself in text and spread
/// position, the `$ref` entity when a selector is present.
pub(super) fn apply_binding(
	binding: &BindingExpr,
	entity: &mut EntityWorldMut,
	comp_target: BindingTarget,
) -> Result<()> {
	match binding.source {
		BindingSource::Doc => {
			entity
				.insert(field_ref(&binding.field_path, binding.init.as_ref())?);
		}
		BindingSource::Prop => {
			entity.insert(
				FieldRef::new(binding.field_path.clone())
					.with_document(DocumentPath::Props),
			);
		}
		// the `Value`<->reflect bridge needs `serde_json` (the `json` feature);
		// an embedded build without it keeps the `Value` but loses the
		// resource/component sync (Risk: documented, acceptable).
		BindingSource::Res => {
			insert_value_if_missing(entity);
			#[cfg(feature = "json")]
			entity.insert(ResourceFieldRef::new(
				binding.type_path.clone().unwrap_or_default(),
				binding.field_path.to_string(),
			));
		}
		BindingSource::Comp => {
			insert_value_if_missing(entity);
			#[cfg(feature = "json")]
			{
				let mut reflect_ref = ReflectFieldRef::new(
					binding.type_path.clone().unwrap_or_default(),
					binding.field_path.to_string(),
				);
				reflect_ref.target = comp_target;
				entity.insert(reflect_ref);
			}
			#[cfg(not(feature = "json"))]
			let _ = comp_target;
		}
	}
	Ok(())
}

/// Seed a default [`Value`] for a binding's sync to fill, preserving any value
/// already present (eg a `FieldRef`-seeded one).
fn insert_value_if_missing(entity: &mut EntityWorldMut) {
	if !entity.contains::<Value>() {
		entity.insert(Value::default());
	}
}

/// The target of an `@entity:name::` selector in a cx-bearing position (text,
/// event): a reserved name resolves to its well-known entity (or defers to the
/// sync pass), else through the `bx:ref` machinery.
fn selector_target(
	name: &SmolStr,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> BindingTarget {
	match ReservedRef::parse(name) {
		Some(reserved) => reserved.target(name, cx),
		None => BindingTarget::Entity(resolve_ref(name, refs, cx)),
	}
}

/// The target of an `@entity:name::` selector resolved against a pre-built
/// name->entity map ([`resolve_entity_refs`], which also resolves the
/// build-time reserved names): a lazy reserved name defers to the sync pass,
/// anything else looks up the map.
fn map_selector_target(
	name: &SmolStr,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> BindingTarget {
	match ReservedRef::parse(name) {
		Some(ReservedRef::PageRoot | ReservedRef::Router) => {
			BindingTarget::Reserved(name.clone())
		}
		_ => BindingTarget::Entity(
			entity_refs
				.get(name)
				.copied()
				.unwrap_or(Entity::PLACEHOLDER),
		),
	}
}

/// The `@comp` target of an attribute-position expression: the `$ref` entity
/// when selected, else the `element` carrying the attribute.
fn attr_comp_target(
	expr: &ValueExpr,
	element: Entity,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> BindingTarget {
	match expr {
		ValueExpr::Binding(BindingExpr {
			selector: Some(name),
			..
		}) => map_selector_target(name, entity_refs),
		_ => BindingTarget::Entity(element),
	}
}

/// Build an element: dispatch on its tag kind, then directives, attributes, and
/// children.
fn build_element(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	if el.tag == "Slot" {
		return build_slot(el, cx.entity);
	}
	if is_uppercase_tag(&el.tag) {
		return build_uppercase(el, registry, refs, cx);
	}
	build_html_element(el, registry, refs, cx)
}

/// Whether a tag resolves by name (a component or template) rather than as an
/// HTML element: a capitalized tag, or a `path::to::X` module path whose final
/// segment is capitalized (a `<path::to::X>` BSX template).
fn is_uppercase_tag(tag: &str) -> bool {
	tag.rsplit("::")
		.next()
		.unwrap_or(tag)
		.starts_with(|ch: char| ch.is_uppercase())
}

/// Build a lowercase HTML element: `Element` + directive components + attribute
/// child entities + child node entities.
fn build_html_element(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	cx.entity.insert(Element::new(el.tag.clone()));
	apply_common_directives(el, refs, cx)?;
	// pre-resolve `$name` refs (forward-aware) so spreads have a plain lookup.
	let entity_refs = resolve_entity_refs(el, refs, cx);
	apply_attributes(el, cx.entity, &entity_refs)?;
	build_children(el, registry, refs, cx)
}

/// Resolve every `$name` referenced by `el`'s attribute literals to a real,
/// forward-mapped entity through the one entity model, keyed by name for a plain
/// lookup during reflect-patch building.
fn resolve_entity_refs(
	el: &BsxElement,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> HashMap<SmolStr, Entity> {
	let mut out = HashMap::default();
	let mut names = Vec::new();
	for attr in &el.attributes {
		match &attr.value {
			AttrValue::Spread(spread) => {
				collect_entity_ref_names(spread, &mut names)
			}
			AttrValue::Expr(ValueExpr::Literal(literal)) => {
				collect_literal_entity_ref_names(literal, &mut names)
			}
			AttrValue::Expr(ValueExpr::EntityRef(name)) => {
				names.push(name.clone())
			}
			// an `@entity:ref::` selector resolves through the same machinery.
			AttrValue::Expr(ValueExpr::Binding(binding)) => {
				names.extend(binding.selector.clone());
			}
			_ => {}
		}
	}
	for name in names {
		// a reserved name never resolves through the `bx:ref` machinery: the
		// build-time ones resolve from the build resources here, the lazy ones
		// (`PageRoot`/`Router`) defer to the sync pass instead.
		if let Some(reserved) = ReservedRef::parse(&name) {
			let fallback = cx.entity.id();
			if let Some(entity) = cx.entity.world_scope(|world| {
				reserved.build_time_entity(world, fallback)
			}) {
				out.insert(name, entity);
			}
			continue;
		}
		let reference =
			refs.get(&name).unwrap_or_else(|| stable_reference(&name));
		// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
		let world = unsafe { cx.entity.world_mut() };
		let entity = cx.entity_references.get(reference, world);
		out.insert(name, entity);
	}
	out
}

/// Collect every `$name` in a spread's literals and binding selectors.
fn collect_entity_ref_names(spread: &SpreadExpr, out: &mut Vec<SmolStr>) {
	match spread {
		SpreadExpr::Named(named) => collect_literal_entity_ref_names(
			&DataLiteral::Enum(named.clone()),
			out,
		),
		SpreadExpr::Tuple(items) => {
			for item in items {
				match item {
					SpreadItem::Named(named) => {
						collect_literal_entity_ref_names(
							&DataLiteral::Enum(named.clone()),
							out,
						)
					}
					SpreadItem::Binding(binding) => {
						out.extend(binding.selector.clone())
					}
				}
			}
		}
	}
}

/// Collect every `$name` referenced anywhere inside a literal.
fn collect_literal_entity_ref_names(
	literal: &DataLiteral,
	out: &mut Vec<SmolStr>,
) {
	match literal {
		DataLiteral::EntityRef(name) => out.push(name.clone()),
		DataLiteral::List(items) => items
			.iter()
			.for_each(|item| collect_literal_entity_ref_names(item, out)),
		DataLiteral::Struct(fields) => fields
			.iter()
			.for_each(|(_, item)| collect_literal_entity_ref_names(item, out)),
		DataLiteral::Enum(named) => match &named.fields {
			NamedFields::Tuple(items) => items
				.iter()
				.for_each(|item| collect_literal_entity_ref_names(item, out)),
			NamedFields::Struct(fields) => {
				fields.iter().for_each(|(_, item)| {
					collect_literal_entity_ref_names(item, out)
				})
			}
			NamedFields::Unit => {}
		},
		DataLiteral::Scalar(_) => {}
	}
}

/// A stable [`SceneEntityReference`] for a `$name` with no declared `bx:ref`, so
/// a dangling reference resolves to a consistent placeholder rather than erroring.
fn stable_reference(name: &str) -> SceneEntityReference {
	let hash = name.bytes().fold(0u64, |acc, byte| {
		acc.wrapping_mul(31).wrapping_add(byte as u64)
	});
	SceneEntityReference::new(("bsx_ref", 0, 0), hash as usize)
}

/// Build a capitalized tag: a component (reflect-patched) or a template (built
/// with input props), incl `<path::to::X>` BSX templates.
fn build_uppercase(
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
	// fetch into the root's pending set, awaited by `LoadTemplate`. Remote
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
		// a remote schema resolves asynchronously, deferring `LoadTemplate`.
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
		registry
			.get_with_short_type_path(&el.tag)
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
		// compiled out) resolves to nothing instead of erroring.
		if is_allowed_unregistered(cx, &el.tag) {
			return Ok(());
		}
		bevybail!(
			"no component, resource or template registered for tag `{}`",
			el.tag
		);
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
			// a component: reflect-patch over default and insert.
			insert_component(cx.entity, patch.as_ref(), &app_registry)?;
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

/// How an uppercase tag's type registration resolves.
enum UppercaseKind {
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

/// Build a `<Slot>` placeholder into `entity` as a [`SlotTarget`] marker, with
/// its children as fallback content. A named slot targets by name; a transfer
/// `<Slot name bx:slot=..>` also carries a [`SlotChild`].
fn build_slot(el: &BsxElement, entity: &mut EntityWorldMut) -> Result<()> {
	let name = string_attr(el, "name");
	match &name {
		Some(name) => entity.insert(SlotTarget::named(name.clone())),
		None => entity.insert(SlotTarget::new()),
	};
	// a transfer: route this slot's content into a parent slot.
	if let Some(slot) = slot_routing(el) {
		entity.insert(slot);
	}
	// fallback children spawn beneath the target.
	let id = entity.id();
	if !el.children.is_empty() {
		entity.world_scope(|world| -> Result<()> {
			let mut references =
				bevy::ecs::template::SceneEntityReferences::default();
			let mut entity_mut = world.entity_mut(id);
			let mut cx = TemplateContext::new(&mut entity_mut, &mut references);
			let refs = RefBindings::default();
			let registry = BsxTemplateRegistry::default();
			for child in &el.children {
				spawn_child(child, id, &registry, &refs, &mut cx)?;
			}
			Ok(())
		})?;
	}
	Ok(())
}

/// Apply the `bx:scope`/`bx:for`/`bx:key`/`bx:ref`/`bx:click`/`slot` directives
/// shared by every tag kind onto `cx.entity`.
fn apply_common_directives(
	el: &BsxElement,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	// `bx:ref` names this entity: bind the pinned reference to it.
	if let Some(name) = string_attr(el, "bx:ref") {
		if let Some(reference) = refs.get(&name) {
			let id = cx.entity.id();
			cx.entity_references.set(reference, id);
		}
	}
	// `bx:scope` -> a `DocumentScope` prefix.
	if let Some(scope) = string_attr(el, "bx:scope") {
		cx.entity.insert(DocumentScope {
			path: FieldPath::new([scope]),
			terminate: false,
		});
	}
	// a `slot`/`bx:slot` routes this whole node into a parent slot.
	if let Some(slot) = slot_routing(el) {
		cx.entity.insert(slot);
	}
	// `bx:style="prop=value .."` declares a one-off rule and attaches a unique,
	// span-derived class, the markup twin of `inline_class!`. The declaration
	// grammar lives in a higher crate, so it resolves through the `StyleResolver`
	// seam (a graceful no-op when no handler is registered).
	if let Some((source, span)) = bsx_style_attr(el) {
		let handler = cx
			.entity
			.world_scope(|world| world.get_resource::<StyleResolver>()?.get());
		if let Some(handler) = handler {
			handler(cx.entity, source, span)?;
		}
	}
	// `bx:<event>=verb{ arg: value, .. }` events. The event name is the directive
	// suffix after `bx:`; the verb + args resolve through the core registries.
	for attr in &el.attributes {
		if !is_event_directive(&attr.key) {
			continue;
		}
		let AttrValue::Verb(call) = &attr.value else {
			bevybail!(
				"`{}` expects a verb call, ie `{}=increment{{ field: @doc:count }}`",
				attr.key,
				attr.key
			);
		};
		let event = attr.key.strip_prefix("bx:").unwrap_or(&attr.key);
		let binding = EventBinding::new(event, call.clone());
		// pre-resolve each binding argument's `@entity:ref::` selector to its target
		// (needs `cx`), so the install closure is a plain lookup with no `cx`
		// borrow conflicting with `cx.entity`.
		let targets = binding
			.args
			.iter()
			.filter_map(|(_, arg)| match arg {
				VerbArg::Binding(BindingExpr {
					selector: Some(name),
					..
				}) => Some((name.clone(), selector_target(name, refs, cx))),
				_ => None,
			})
			.collect::<HashMap<SmolStr, BindingTarget>>();
		install_event(cx.entity, &binding, |selector| {
			selector
				.and_then(|name| targets.get(name).cloned())
				.unwrap_or(BindingTarget::This)
		})?;
	}
	Ok(())
}

/// Build an element's child nodes. A `bx:for` element instead materializes a
/// reactive list, one child per item over the named array field.
fn build_children(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	if let Some(field) = string_attr(el, "bx:for") {
		return build_reactive_children(el, &field, registry, refs, cx);
	}
	let parent = cx.entity.id();
	for child in &el.children {
		spawn_child(child, parent, registry, refs, cx)?;
	}
	Ok(())
}

/// `bx:for="items"` + `bx:key`: a [`ReactiveChildren`] over the `items` array,
/// spawning the element's child template per item, each in a terminating index
/// scope.
fn build_reactive_children(
	el: &BsxElement,
	field: &str,
	registry: &BsxTemplateRegistry,
	_refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	let template = el.children.clone();
	let registry = registry.clone();
	// the field backing the list: its synced `Value` drives the rebuild.
	cx.entity.insert(FieldRef::new(field));
	cx.entity
		.insert(ReactiveChildren::new(move |_index, _item| {
			let template = template.clone();
			let registry = registry.clone();
			OnSpawn::new(move |entity| {
				// each item child builds the element's body as its own template; the
				// terminating index scope `ReactiveChildren` adds resolves its fields.
				let nested =
					BsxTemplate::container(template.clone(), registry.clone());
				if let Err(error) = entity.build_template(&nested) {
					entity.insert(TemplateError::new(error));
				}
			})
		}));
	Ok(())
}

/// Build the caller content of an uppercase tag as slot children: each child its
/// own entity carrying a default [`SlotChild`] unless it routes itself.
fn build_slot_children(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	let parent = cx.entity.id();
	for child in &el.children {
		let entity = spawn_child(child, parent, registry, refs, cx)?;
		// an unrouted child goes to the default slot.
		if !child_routes_itself(child) {
			// SAFETY: only used to mark the spawned slot child.
			let world = unsafe { cx.entity.world_mut() };
			if !world.entity(entity).contains::<SlotChild>() {
				world.entity_mut(entity).insert(SlotChild::new());
			}
		}
	}
	Ok(())
}

/// Whether a child element carries its own `slot`/`bx:slot` routing.
fn child_routes_itself(node: &BsxNode) -> bool {
	matches!(node, BsxNode::Element(el) if slot_routing(el).is_some())
}

/// Apply an element's attributes as attribute child entities, the
/// `Element` + `related!(Attributes[..])` shape `rsx!` produces.
fn apply_attributes(
	el: &BsxElement,
	entity: &mut EntityWorldMut,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let parent = entity.id();
	for attr in &el.attributes {
		// directives and spreads are not HTML attributes.
		if is_directive(&attr.key) || attr.key.is_empty() {
			if let AttrValue::Spread(_) = &attr.value {
				// spreads handled separately below.
			}
			continue;
		}
		// `on*` event handlers are not implemented as data attributes here.
		entity.world_scope(|world| -> Result<()> {
			let mut attr_entity = world.spawn((
				AttributeOf::new(parent),
				Attribute::new(attr.key.clone()),
			));
			match &attr.value {
				AttrValue::Flag => {}
				AttrValue::Str(string) => {
					attr_entity.insert(Value::Str(string.into()));
				}
				AttrValue::Expr(expr) => {
					// attribute position: an `@comp` binds the element unless
					// `$ref` retargets.
					let comp_target =
						attr_comp_target(expr, parent, entity_refs);
					apply_value_expr(expr, &mut attr_entity, comp_target)?;
				}
				// spreads, `bx:<event>` verb calls and `bx:style` are handled
				// elsewhere (the directives pass / spread pass).
				AttrValue::Spread(_)
				| AttrValue::Verb(_)
				| AttrValue::Style { .. } => {}
			}
			Ok(())
		})?;
	}
	apply_spreads(el, entity, entity_refs)
}

/// Insert every bare-position spread's components/templates onto `entity`,
/// shared by every tag kind (an HTML element, a component, a template). The
/// `AppTypeRegistry` is only touched when a spread is present, so a plain
/// HTML/markdown parse (no registry) never needs it.
fn apply_spreads(
	el: &BsxElement,
	entity: &mut EntityWorldMut,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let has_spread = el
		.attributes
		.iter()
		.any(|attr| matches!(attr.value, AttrValue::Spread(_)));
	if !has_spread {
		return Ok(());
	}
	let app_registry = entity
		.world_scope(|world| world.get_resource::<AppTypeRegistry>().cloned())
		.ok_or_else(|| {
			bevyhow!("a spread requires an `AppTypeRegistry` in the world")
		})?;
	for attr in &el.attributes {
		if let AttrValue::Spread(spread) = &attr.value {
			apply_spread(spread, entity, &app_registry, entity_refs)?;
		}
	}
	Ok(())
}

/// Insert or build a spread's components/templates onto `entity`. A tuple's
/// `@` binding items apply to the same entity, pairing a component insert with
/// its binding, eg `{(Bar{boo:"bazz"}, @comp:Bar.boo)}`.
fn apply_spread(
	spread: &SpreadExpr,
	entity: &mut EntityWorldMut,
	app_registry: &AppTypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	match spread {
		SpreadExpr::Named(named) => {
			apply_spread_named(named, entity, app_registry, entity_refs)
		}
		SpreadExpr::Tuple(items) => {
			for item in items {
				match item {
					SpreadItem::Named(named) => apply_spread_named(
						named,
						entity,
						app_registry,
						entity_refs,
					)?,
					// spread position: an `@comp` binds this entity unless
					// `$ref` retargets.
					SpreadItem::Binding(binding) => {
						let comp_target = match &binding.selector {
							Some(name) => {
								map_selector_target(name, entity_refs)
							}
							None => BindingTarget::This,
						};
						apply_binding(binding, entity, comp_target)?;
					}
				}
			}
			Ok(())
		}
	}
}

/// Insert or build one named spread component/template onto `entity`.
///
/// A name with no registered type is a capability this binary did not link (eg a
/// `<Router {(.., TuiServer)}>` spread loaded by a lean http-only deploy that
/// dropped the `tui` feature). Skip it with a warning rather than failing the
/// whole load, so the same site serves the subset each binary supports.
fn apply_spread_named(
	named: &NamedLiteral,
	entity: &mut EntityWorldMut,
	app_registry: &AppTypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let literal = DataLiteral::Enum(named.clone());
	let (is_template, patch) = {
		let registry = app_registry.read();
		let Some(registration) = registry.get_with_short_type_path(&named.name)
		else {
			warn!(
				"skipping spread `{}`: no component or template of that name is registered in this binary",
				named.name
			);
			return Ok(());
		};
		let info = Some(registration.type_info());
		let is_template = registration.data::<ReflectTemplate>().is_some();
		let mut resolver = entity_ref_resolver(entity_refs);
		(
			is_template,
			literal_to_reflect(&literal, info, &registry, &mut resolver)?,
		)
	};
	if is_template {
		let id = entity.id();
		entity.world_scope(|world| -> Result<()> {
			let mut references =
				bevy::ecs::template::SceneEntityReferences::default();
			let mut entity_mut = world.entity_mut(id);
			let mut cx = TemplateContext::new(&mut entity_mut, &mut references);
			build_template_by_name(
				app_registry,
				&named.name,
				patch.as_ref(),
				&mut cx,
			)
		})?;
	} else {
		insert_component(entity, patch.as_ref(), app_registry)?;
	}
	Ok(())
}

/// Build a reflect patch for an uppercase tag's attributes against its type
/// info, so each value coerces to the field's concrete type.
fn build_patch(
	el: &BsxElement,
	type_info: &'static bevy::reflect::TypeInfo,
	registry: &TypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<Box<dyn bevy::reflect::PartialReflect>> {
	use bevy::reflect::structs::DynamicStruct;
	let struct_info = match type_info {
		bevy::reflect::TypeInfo::Struct(info) => Some(info),
		_ => None,
	};
	let mut patch = DynamicStruct::default();
	for attr in &el.attributes {
		if is_directive(&attr.key) || attr.key.is_empty() {
			continue;
		}
		// an `@` binding becomes a field binding, not a patch field.
		if matches!(&attr.value, AttrValue::Expr(ValueExpr::Binding(_))) {
			continue;
		}
		let field_info = struct_info
			.and_then(|info| info.field(&attr.key))
			.and_then(|field| field.type_info());
		let literal = attr_to_literal(&attr.value)?;
		let mut resolver = entity_ref_resolver(entity_refs);
		// a `#[template]` stores an optional/required prop as `PropOpt<T>`; a markup
		// value must wrap into `PropOpt(Some(value))` to apply over the default.
		let reflected = match prop_opt_inner_info(field_info) {
			Some(inner_info) => prop_opt_value(
				&literal,
				field_info,
				inner_info,
				registry,
				&mut resolver,
			)?,
			None => literal_to_reflect(
				&literal,
				field_info,
				registry,
				&mut resolver,
			)?,
		};
		patch.insert_boxed(&attr.key, reflected);
	}
	patch.set_represented_type(Some(type_info));
	Ok(Box::new(patch))
}

/// If `field_info` is a `PropOpt<T>` tuple struct, the inner `Option<T>`'s
/// [`TypeInfo`], else `None`. A `#[template]`'s optional/required props store as
/// `PropOpt<T>`, so a markup value targeting one must wrap into the option.
fn prop_opt_inner_info(
	field_info: Option<&'static bevy::reflect::TypeInfo>,
) -> Option<&'static bevy::reflect::TypeInfo> {
	let bevy::reflect::TypeInfo::TupleStruct(info) = field_info? else {
		return None;
	};
	if !info.type_path().contains("PropOpt<") {
		return None;
	}
	info.field_at(0).and_then(|field| field.type_info())
}

/// Build a `PropOpt(Some(value))` reflected value for a `PropOpt<T>` field, so a
/// markup prop value reaches a `#[template]`'s optional/required prop.
fn prop_opt_value(
	literal: &DataLiteral,
	field_info: Option<&'static bevy::reflect::TypeInfo>,
	option_info: &'static bevy::reflect::TypeInfo,
	registry: &TypeRegistry,
	resolver: &mut dyn FnMut(&str) -> Entity,
) -> Result<Box<dyn bevy::reflect::PartialReflect>> {
	use bevy::reflect::enums::DynamicEnum;
	use bevy::reflect::enums::DynamicVariant;
	use bevy::reflect::enums::VariantInfo;
	use bevy::reflect::tuple::DynamicTuple;
	use bevy::reflect::tuple_struct::DynamicTupleStruct;
	// the `Option<T>` carried by `PropOpt<T>(Option<T>)`; resolve the inner `T`.
	let inner_info = match option_info {
		bevy::reflect::TypeInfo::Enum(enum_info) => enum_info
			.variant("Some")
			.and_then(|variant| match variant {
				VariantInfo::Tuple(tuple) => tuple.field_at(0),
				_ => None,
			})
			.and_then(|field| field.type_info()),
		_ => None,
	};
	let inner = literal_to_reflect(literal, inner_info, registry, resolver)?;
	// `Some(inner)`
	let mut some = DynamicTuple::default();
	some.insert_boxed(inner);
	let mut option = DynamicEnum::new("Some", DynamicVariant::Tuple(some));
	option.set_represented_type(Some(option_info));
	// `PropOpt(Some(inner))`
	let mut prop_opt = DynamicTupleStruct::default();
	prop_opt.insert_boxed(Box::new(option));
	prop_opt.set_represented_type(field_info);
	Ok(Box::new(prop_opt))
}

/// An [`EntityResolver`] over a pre-resolved name->entity map; an unknown name
/// falls back to a placeholder so a dangling `$name` never panics.
fn entity_ref_resolver(
	entity_refs: &HashMap<SmolStr, Entity>,
) -> impl FnMut(&str) -> Entity + '_ {
	move |name| {
		entity_refs
			.get(name)
			.copied()
			.unwrap_or(Entity::PLACEHOLDER)
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
	use bevy::ecs::reflect::ReflectComponent;
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
					|| registration
						.data::<ReflectFromReflect>()
						.is_some_and(|from_reflect| {
							from_reflect.from_reflect(patch).is_some()
						});
				if !constructible {
					bevybail!(
						"`<{}>`: the resource is not in the world and `{}` cannot be constructed from the patch, add `#[reflect(Default)]` or insert the resource first",
						el.tag,
						type_info.type_path()
					);
				}
				let resource_entity = world.spawn_empty().id();
				reflect_component.insert(
					&mut world.entity_mut(resource_entity),
					patch,
					&registry,
				);
			}
		}
		Ok(())
	})
}

/// Insert a reflect-patched component over its default onto `entity`.
fn insert_component(
	entity: &mut EntityWorldMut,
	patch: &dyn bevy::reflect::PartialReflect,
	app_registry: &AppTypeRegistry,
) -> Result<()> {
	use bevy::ecs::reflect::ReflectComponent;
	let registry = app_registry.read();
	let type_info = patch.get_represented_type_info().ok_or_else(|| {
		bevyhow!("spread/component patch has no represented type")
	})?;
	let registration = registry.get(type_info.type_id()).ok_or_else(|| {
		bevyhow!("type `{}` is not registered", type_info.type_path())
	})?;
	let reflect_component =
		registration.data::<ReflectComponent>().ok_or_else(|| {
			bevyhow!(
				"type `{}` is not a registered component",
				type_info.type_path()
			)
		})?;
	// `from_reflect` the partial patch over default, then insert. A `DynamicStruct`
	// carrying only the provided fields fills the rest from the type's default.
	reflect_component.insert(entity, patch, &registry);
	Ok(())
}

// --- value/attribute lowering helpers ----------------------------------------

/// Lower a literal in text/attribute position to a concrete [`Value`].
fn literal_to_value(literal: &DataLiteral) -> Result<Value> {
	match literal {
		DataLiteral::Scalar(value) => Ok(value.clone()),
		DataLiteral::List(items) => items
			.iter()
			.map(literal_to_value)
			.collect::<Result<Vec<_>>>()
			.map(Value::List),
		DataLiteral::Struct(fields) => {
			let mut map = Map::default();
			for (key, value) in fields {
				map.insert(key.clone(), literal_to_value(value)?);
			}
			Ok(Value::Map(map))
		}
		DataLiteral::Enum(named)
			if matches!(named.fields, NamedFields::Unit) =>
		{
			Ok(Value::Str(named.name.clone().into()))
		}
		DataLiteral::Enum(_) => {
			bevybail!("enum literal with fields is not a plain text value")
		}
		DataLiteral::EntityRef(_) => {
			bevybail!("`$name` entity references are not a plain text value")
		}
	}
}

/// Build a [`FieldRef`] from a `@doc:field=init` binding.
fn field_ref(path: &FieldPath, init: Option<&DataLiteral>) -> Result<FieldRef> {
	let mut field = FieldRef::new(path.clone());
	if let Some(init) = init {
		field = field.with_init(literal_to_value(init)?);
	}
	Ok(field)
}

/// Lower an attribute value to a literal for reflect-patching an uppercase tag.
fn attr_to_literal(value: &AttrValue) -> Result<DataLiteral> {
	match value {
		AttrValue::Flag => Ok(DataLiteral::Scalar(Value::Bool(true))),
		AttrValue::Str(string) => {
			Ok(DataLiteral::Scalar(Value::Str(string.into())))
		}
		AttrValue::Expr(ValueExpr::Literal(literal)) => Ok(literal.clone()),
		// `field=$name` lowers to an entity-reference literal on the patched field.
		AttrValue::Expr(ValueExpr::EntityRef(name)) => {
			Ok(DataLiteral::EntityRef(name.clone()))
		}
		AttrValue::Expr(ValueExpr::Binding(_)) => {
			bevybail!("an `@` binding is not a component patch value")
		}
		AttrValue::Spread(_) => bevybail!("a spread is not an attribute value"),
		AttrValue::Verb(_) => {
			bevybail!("a `bx:<event>` verb call is not a component patch value")
		}
		AttrValue::Style { .. } => {
			bevybail!("a `bx:style` directive is not a component patch value")
		}
	}
}

// --- attribute lookup helpers ------------------------------------------------

/// The string value of a literal-string attribute, if present.
fn string_attr(el: &BsxElement, key: &str) -> Option<String> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != key {
			return None;
		}
		match &attr.value {
			AttrValue::Str(string) => Some(string.clone()),
			AttrValue::Flag => Some(String::new()),
			_ => None,
		}
	})
}

/// The [`SlotChild`] routing marker for an element's `slot`/`bx:slot`, if any.
fn slot_routing(el: &BsxElement) -> Option<SlotChild> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != "slot" && attr.key != "bx:slot" {
			return None;
		}
		match &attr.value {
			AttrValue::Str(name) => Some(SlotChild::named(name.clone())),
			_ => Some(SlotChild::new()),
		}
	})
}

/// Whether a key is a `bx:`/slot directive rather than an HTML attribute.
pub(super) fn is_directive(key: &str) -> bool {
	key.starts_with("bx:") || key == "slot"
}

/// The `bx:` directives with dedicated structural meaning, as opposed to a
/// `bx:<event>` verb trigger. Anything else under `bx:` is treated as an event
/// (resolved through the [`EventRegistry`], a graceful no-op when unregistered).
const STRUCTURAL_DIRECTIVES: &[&str] = &[
	"bx:scope",
	"bx:for",
	"bx:key",
	"bx:slot",
	"bx:ref",
	"bx:schema",
	"bx:style",
];

/// Whether a key is a `bx:<event>` verb-trigger directive (eg `bx:click`), ie a
/// `bx:` key that is not one of the [`STRUCTURAL_DIRECTIVES`].
pub(super) fn is_event_directive(key: &str) -> bool {
	key.starts_with("bx:") && !STRUCTURAL_DIRECTIVES.contains(&key)
}
