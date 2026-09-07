//! The entity model of a build: `bx:ref` names, the reserved selectors they may
//! not shadow, and the two-pass `$name` resolution that lets a reference point
//! forward.

use crate::prelude::*;
use bevy::ecs::template::SceneEntityReference;
use bevy::ecs::template::TemplateContext;

/// The root entity of the innermost [`BsxTemplate`] build (the parsed document
/// container or a registry `<path::to::X>` body): the `SnippetRoot` reserved
/// target. Set for the duration of a build, mirroring [`TemplateBuildRoot`].
#[derive(Debug, Clone, Copy, Resource)]
pub(super) struct SnippetBuildRoot(pub(super) Entity);

/// Names declared by a `bx:ref` anywhere in the tree, each pinned to a stable
/// [`SceneEntityReference`] so a forward `$name` resolves identically.
#[derive(Default)]
pub(super) struct RefBindings {
	names: HashMap<SmolStr, SceneEntityReference>,
}

impl RefBindings {
	/// The pinned reference for `name`, allocating a stable one on first use.
	fn reference(&mut self, name: &str) -> SceneEntityReference {
		let next = self.names.len();
		*self.names.entry(name.into()).or_insert_with(|| {
			// deterministic ref (no runtime disambiguator): identity is the pinned name index.
			SceneEntityReference::new(("bsx_ref", 0, 0), next, 0)
		})
	}

	/// The pinned reference for `name`, if declared by a `bx:ref`.
	pub(super) fn get(&self, name: &str) -> Option<SceneEntityReference> {
		self.names.get(name).copied()
	}
}

/// Walk the tree collecting every `bx:ref` name into stable references,
/// erroring on a name that shadows a reserved selector ([`ReservedRef`]).
pub(super) fn collect_refs(
	nodes: &[BsxNode],
	refs: &mut RefBindings,
) -> Result<()> {
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
pub(super) enum ReservedRef {
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
	pub(super) fn target(
		self,
		name: &SmolStr,
		cx: &mut TemplateContext,
	) -> BindingTarget {
		let fallback = cx.entity.id();
		cx.entity
			.world_scope(|world| self.build_time_entity(world, fallback))
			.map(BindingTarget::Entity)
			.unwrap_or_else(|| BindingTarget::Reserved(name.clone()))
	}
}

/// Resolve a `$name` to its real, forward-mapped entity through the one entity
/// model, spawning the pinned placeholder on first use.
pub(super) fn resolve_ref(
	name: &str,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Entity {
	let reference = refs.get(name).unwrap_or_else(|| stable_reference(name));
	// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
	let world = unsafe { cx.entity.world_mut() };
	cx.entity_references.get(reference, world)
}

/// Resolve every `$name` referenced by `el`'s attribute literals to a real,
/// forward-mapped entity through the one entity model, keyed by name for a plain
/// lookup during reflect-patch building.
pub(super) fn resolve_entity_refs(
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
	// a tag-position literal resolves its `$name` refs like a spread, so eg a
	// `<GoTo($target)/>` field maps to the named entity.
	if let Some(named) = &el.tag_literal {
		collect_literal_entity_ref_names(
			&DataLiteral::Enum(named.clone()),
			&mut names,
		);
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
	SceneEntityReference::new(("bsx_ref", 0, 0), hash as usize, 0)
}

/// An [`EntityResolver`] over a pre-resolved name->entity map; an unknown name
/// falls back to a placeholder so a dangling `$name` never panics.
pub(super) fn entity_ref_resolver(
	entity_refs: &HashMap<SmolStr, Entity>,
) -> impl FnMut(&str) -> Entity + '_ {
	move |name| {
		entity_refs
			.get(name)
			.copied()
			.unwrap_or(Entity::PLACEHOLDER)
	}
}
