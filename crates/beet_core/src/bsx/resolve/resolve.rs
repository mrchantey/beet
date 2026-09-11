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
use super::binding::*;
use super::build_cfg::*;
use super::element::*;
use super::entity_refs::*;
use crate::prelude::*;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;

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
		// pass 0: `bx:cfg` exclusion, before anything looks at the tree. An
		// excluded branch is removed from the SYNTAX, so no later pass can
		// observe it: no pinned `bx:ref`, no entity, no build-time effect. Only
		// a document that declares a condition pays for the walk.
		let pruned = if contains_build_cfg(&self.nodes) {
			// SAFETY: reads the condition seam and crate registrations, no flush.
			let world = unsafe { cx.entity.world_mut() };
			Some(prune_build_cfg(&self.nodes, world)?)
		} else {
			None
		};
		let (nodes, excluded) = match &pruned {
			Some((nodes, excluded)) => (nodes.as_slice(), excluded.clone()),
			None => (self.nodes.as_slice(), default()),
		};
		// pass 1: collect every `bx:ref` name -> a pinned reference id, so a `$name`
		// forward reference resolves to the same placeholder entity; a name the
		// exclusion above removed is remembered so reaching for it warns.
		let mut refs = RefBindings::default().with_excluded(excluded);
		collect_refs(nodes, &mut refs)?;
		// expose this build's root for `@entity:SnippetRoot::`, restoring any outer
		// snippet root so nested registry-template builds nest correctly.
		let root = cx.entity.id();
		// SAFETY: only used to swap the snippet-root resource, no flush.
		let world = unsafe { cx.entity.world_mut() };
		let previous = world.remove_resource::<SnippetBuildRoot>();
		world.insert_resource(SnippetBuildRoot(root));
		let result = if self.as_container {
			// every root node spawns as a child of the container entity.
			nodes.iter().try_for_each(|node| {
				spawn_child(node, root, &self.registry, &refs, cx).map(|_| ())
			})
		} else {
			build_root_nodes(nodes, &self.registry, &refs, cx)
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
pub(super) fn spawn_child(
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

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A tuple-struct component with an `Entity` field, the `StoreRef` shape a
	/// declaration is named by.
	#[derive(Component, Reflect, MapEntities, Clone, Debug, PartialEq)]
	#[reflect(Component, MapEntities, Default)]
	struct Bound(#[entities] Entity);

	impl Default for Bound {
		fn default() -> Self { Self(Entity::PLACEHOLDER) }
	}

	/// Build `markup` into a world registering `PackageConfig` and [`Bound`],
	/// returning the world and the built root.
	fn build(markup: &str) -> (World, Entity) {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		{
			let registry = world.resource_mut::<AppTypeRegistry>();
			let mut registry = registry.write();
			registry.register::<PackageConfig>();
			registry.register::<Bound>();
			registry.register::<RequireCfg>();
		}
		let nodes =
			BsxNode::parse_document(markup, &BsxParseConfig::bsx()).unwrap();
		let root = world
			.spawn_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.flush();
		(world, root)
	}

	/// The entities `markup` builds, root included.
	fn entity_count(markup: &str) -> usize {
		let (mut world, root) = build(markup);
		world.with_state::<Query<&Children>, _>(|children| {
			children.iter_descendants_inclusive(root).count()
		})
	}

	/// Structure is universal: an unresolvable tag is an inert entity recording
	/// the name it named, and its children build under it exactly as they would
	/// in a binary that linked the type.
	#[crate::test]
	fn unregistered_tag_is_inert_and_builds_children() {
		let (world, root) =
			build("<NotRegistered><PackageConfig/></NotRegistered>");
		let inert = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity(inert)
			.get::<UnregisteredTag>()
			.unwrap()
			.as_str()
			.xpect_eq("NotRegistered");
		// the child built, and the resource declaration it carries applied.
		world
			.entity(inert)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(1);
		// the same shape a registered tag would build: root, tag, child.
		entity_count("<NotRegistered><PackageConfig/></NotRegistered>")
			.xpect_eq(entity_count("<div><PackageConfig/></div>"));
	}

	/// A string attribute fills a field, the shape an entry declares its cfg
	/// requirement in.
	#[crate::test]
	fn attribute_fills_a_field() {
		let (world, root) = build(
			r#"<RequireCfg cfg="feature:ml && feature:beet_esp/alvik"/>"#,
		);
		let host = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity(host)
			.get::<RequireCfg>()
			.unwrap()
			.xpect_eq(RequireCfg::new("feature:ml && feature:beet_esp/alvik"));
	}

	/// A `$name` reference into a formerly-gated region resolves to the real
	/// inert entity, not a never-built placeholder.
	#[crate::test]
	fn ref_into_an_unregistered_subtree_resolves() {
		let (world, root) = build(
			r#"<div><NotRegistered bx:ref="target"/><span {Bound($target)}/></div>"#,
		);
		let host = world.entity(root).get::<Children>().unwrap()[0];
		let children = world.entity(host).get::<Children>().unwrap();
		let (target, consumer) = (children[0], children[1]);
		world
			.entity(target)
			.contains::<UnregisteredTag>()
			.xpect_true();
		world
			.entity(consumer)
			.get::<Bound>()
			.unwrap()
			.0
			.xpect_eq(target);
	}

	/// The narrow [`AllowedUnregistered`] opt-out still resolves to nothing at
	/// all, children included: only a tag whose whole content IS the missing
	/// behavior takes it.
	#[crate::test]
	fn allowed_unregistered_tag_builds_nothing() {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		world.allow_unregistered("NotRegistered");
		let nodes = BsxNode::parse_document(
			"<NotRegistered><NorThis/></NotRegistered>",
			&BsxParseConfig::bsx(),
		)
		.unwrap();
		let root = world
			.spawn_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.flush();
		// the tag's own entity is spawned by the walker, but nothing builds into
		// it and no child follows.
		world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(1);
		let inert = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity(inert)
			.contains::<UnregisteredTag>()
			.xpect_false();
		world.entity(inert).get::<Children>().is_none().xpect_true();
	}
}
