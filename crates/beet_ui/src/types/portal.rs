//! Transclusion, a [`Portal`] rendering another entity in place, and
//! [`RenderTreeQuery`], the one traversal over the tree a render sees.
use alloc::collections::VecDeque;
use beet_core::prelude::*;

/// Renders another entity in place, by reference, without reparenting it.
///
/// A holder is transparent: every walk over a rendered tree reads it through
/// [`RenderTreeQuery`], which substitutes the referenced entity for the holder,
/// ignoring the holder's own components and [`Children`]. The referenced entity
/// is neither owned nor moved, so it can be a separately-managed subtree (eg
/// per-request route content) transcluded into a document layout without being
/// owned by it.
///
/// This is distinct from author-facing `<slot>` composition (which lowers to
/// [`SceneProp`] props at macro time): the layout middleware needs to inject
/// already-spawned route content into a freshly-spawned document layout without
/// despawning it, by reference rather than by value.
///
/// The source half of the one-to-many [`PortalOf`] relationship: the holder
/// points at one content entity, and the content tracks every holder that
/// transcludes it (eg a layout root and a live page-host slot rendering the same
/// content). The reverse edge is what lets a binding cross the transclusion
/// boundary (the layout-head `@entity:PageRoot::` walk into the route content)
/// and the cascade inherit through it (the holder is the visual parent of the
/// content). Absence is the unresolved state, eg a page-host slot before any
/// page is set, so a placeholder entity is never exposed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = PortalOf)]
pub struct Portal(#[entities] pub Entity);

impl Portal {
	/// Render `target` in place.
	pub fn new(target: Entity) -> Self { Self(target) }

	/// The referenced entity.
	pub fn target(&self) -> Entity { self.0 }
}

/// The holders that render this entity in place by reference, the target half of
/// the [`Portal`] relationship.
///
/// The reverse edge of a transclusion: content transcluded into a layout (or a
/// live page host) has no [`ChildOf`] link to its holder, so this is how a walk
/// crosses from content up into the holder. A binding in a layout head resolving
/// `@entity:PageRoot::` follows it from the layout root to the route content,
/// and the style cascade inherits from the holder through it.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = Portal)]
pub struct PortalOf(Vec<Entity>);

impl PortalOf {
	/// The holders rendering this entity in place.
	pub fn holders(&self) -> &[Entity] { &self.0 }
}

/// The tree a render sees: [`Children`] downward with each [`Portal`] holder
/// replaced by the entity it transcludes, and [`ChildOf`] upward with each
/// transcluded entity's first [`PortalOf`] holder as its parent.
///
/// Every walk over a rendered tree reads it through this, so a portal is
/// stepped through in one place. A holder renders as its target, so every
/// method reads `entity` as the entity it renders as: a holder's children are
/// its target's, and an iterator yields the target where a holder sits.
/// Neither [`ChildOf`] nor [`Portal`] admits a self-referential edge, so the
/// upward walk always terminates.
///
/// The tags a text walk skips are owned here too:
/// [`is_textless`](Self::is_textless).
#[derive(SystemParam)]
pub struct RenderTreeQuery<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	parents: Query<'w, 's, &'static ChildOf>,
	portals: Query<'w, 's, &'static Portal>,
	holders: Query<'w, 's, &'static PortalOf>,
}

impl RenderTreeQuery<'_, '_> {
	/// Document metadata and scripting, never rendered by any target: a visual
	/// one hides them with `display: none`, a text one skips them.
	pub const METADATA_TAGS: &'static [&'static str] = &[
		"head", "script", "style", "template", "noscript", "meta", "link",
		"title", "base",
	];

	/// Embedded resources and pictures: visual on the web, but no text, so a
	/// text walk skips them (an `<svg>`'s `<text>` is part of a picture, not
	/// prose).
	pub const EMBEDDED_TAGS: &'static [&'static str] = &[
		"iframe", "object", "embed", "svg", "video", "audio", "canvas",
	];

	/// Whether a `tag`'s content is no text a reader reads, ie
	/// [`METADATA_TAGS`](Self::METADATA_TAGS) and
	/// [`EMBEDDED_TAGS`](Self::EMBEDDED_TAGS). A walk reading prose (markdown,
	/// plaintext, Leaflet) skips these subtrees; a markup serializer emits them.
	pub fn is_textless(tag: &str) -> bool {
		Self::METADATA_TAGS.contains(&tag) || Self::EMBEDDED_TAGS.contains(&tag)
	}

	/// The entity `entity` renders as: itself, or for a holder the entity its
	/// [`Portal`] chain ends at.
	pub fn resolve(&self, mut entity: Entity) -> Entity {
		while let Ok(portal) = self.portals.get(entity) {
			entity = portal.target();
		}
		entity
	}

	/// The children of the entity `entity` renders as, in order, each holder
	/// replaced by the entity it renders as.
	pub fn children(
		&self,
		entity: Entity,
	) -> impl '_ + DoubleEndedIterator<Item = Entity> {
		self.children
			.get(self.resolve(entity))
			.into_iter()
			.flat_map(|children| children.iter())
			.map(|child| self.resolve(child))
	}

	/// `root` and every entity under it, breadth-first: the shallowest first,
	/// then in document order.
	pub fn iter_descendants_inclusive(
		&self,
		root: Entity,
	) -> impl '_ + Iterator<Item = Entity> {
		let mut queue = VecDeque::from([self.resolve(root)]);
		core::iter::from_fn(move || {
			let entity = queue.pop_front()?;
			queue.extend(self.children(entity));
			Some(entity)
		})
	}

	/// `root` and every entity under it, depth-first in document order, ie
	/// pre-order.
	pub fn iter_descendants_inclusive_depth_first(
		&self,
		root: Entity,
	) -> impl '_ + Iterator<Item = Entity> {
		let mut stack = vec![self.resolve(root)];
		core::iter::from_fn(move || {
			let entity = stack.pop()?;
			stack.extend(self.children(entity).rev());
			Some(entity)
		})
	}

	/// The parent `entity` renders under: the first holder transcluding it,
	/// else its [`ChildOf`] parent. Transclusion wins, mirroring
	/// [`children`](Self::children) downward, so a walk up from transcluded
	/// content crosses into its holder rather than dead-ending at the content
	/// root.
	pub fn visual_parent(&self, entity: Entity) -> Option<Entity> {
		self.holders
			.get(entity)
			.ok()
			.and_then(|portal_of| portal_of.holders().first().copied())
			.or_else(|| self.parents.get(entity).ok().map(ChildOf::parent))
	}

	/// `entity` and its visual ancestors, innermost first, each hop through
	/// [`visual_parent`](Self::visual_parent).
	pub fn iter_ancestors_inclusive(
		&self,
		entity: Entity,
	) -> impl '_ + Iterator<Item = Entity> {
		core::iter::successors(Some(entity), |entity| {
			self.visual_parent(*entity)
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	fn walker_renders_referenced_entity() {
		let mut world = World::new();

		// content entity: <em>transcluded</em>
		let content = world.spawn(Element::new("em")).id();
		world.spawn((Value::Str("transcluded".into()), ChildOf(content)));

		// holder: a transparent entity that points at the content
		let root = world.spawn(Portal::new(content)).id();

		HtmlRenderer::new()
			.render(&mut RenderContext::new(
				&mut world,
				root,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
			.xpect_contains("<em>transcluded</em>");
	}

	#[beet_core::test]
	fn reverse_edge_tracks_holders() {
		let mut world = World::new();
		let content = world.spawn_empty().id();
		let holder = world.spawn(Portal::new(content)).id();
		// the relationship hook mirrors the holder onto the content's reverse edge.
		world
			.entity(content)
			.get::<PortalOf>()
			.unwrap()
			.holders()
			.xpect_eq(&[holder]);
	}

	/// A root holding `before`, a holder transcluding `content` (itself
	/// holding `first` and `second`), then `after`: `[root, before, holder,
	/// content, first, second, after]`.
	fn transcluded(world: &mut World) -> [Entity; 7] {
		let first = world.spawn_empty().id();
		let second = world.spawn_empty().id();
		let content = world.spawn_empty().add_children(&[first, second]).id();
		let before = world.spawn_empty().id();
		let holder = world.spawn(Portal::new(content)).id();
		let after = world.spawn_empty().id();
		let root = world
			.spawn_empty()
			.add_children(&[before, holder, after])
			.id();
		[root, before, holder, content, first, second, after]
	}

	#[beet_core::test]
	fn children_substitute_the_holder() {
		let mut world = World::new();
		let [root, before, holder, content, first, second, after] =
			transcluded(&mut world);
		world
			.with_state::<RenderTreeQuery, _>(|tree| {
				tree.children(root).collect::<Vec<_>>()
			})
			.xpect_eq(vec![before, content, after]);
		// a holder renders as its target, so its children are the target's
		world
			.with_state::<RenderTreeQuery, _>(|tree| {
				tree.children(holder).collect::<Vec<_>>()
			})
			.xpect_eq(vec![first, second]);
	}

	#[beet_core::test]
	fn descendants_substitute_the_holder_in_both_orders() {
		let mut world = World::new();
		let [root, before, _holder, content, first, second, after] =
			transcluded(&mut world);
		world
			.with_state::<RenderTreeQuery, _>(|tree| {
				tree.iter_descendants_inclusive(root).collect::<Vec<_>>()
			})
			.xpect_eq(vec![root, before, content, after, first, second]);
		world
			.with_state::<RenderTreeQuery, _>(|tree| {
				tree.iter_descendants_inclusive_depth_first(root)
					.collect::<Vec<_>>()
			})
			.xpect_eq(vec![root, before, content, first, second, after]);
	}

	#[beet_core::test]
	fn visual_parent_crosses_the_portal() {
		let mut world = World::new();
		let [root, _before, holder, content, first, ..] =
			transcluded(&mut world);
		world
			.with_state::<RenderTreeQuery, _>(|tree| {
				tree.iter_ancestors_inclusive(first).collect::<Vec<_>>()
			})
			.xpect_eq(vec![first, content, holder, root]);
	}
}
