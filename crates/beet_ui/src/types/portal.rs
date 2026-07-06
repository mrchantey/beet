use crate::prelude::RenderSurface;
use beet_core::prelude::*;

/// Renders another entity in place, by reference, without reparenting it.
///
/// When a [`NodeWalker`] visits an entity carrying this component it recurses
/// into the referenced entity instead of the holder's own components and
/// [`Children`]. The referenced entity is neither owned nor moved, so it can be
/// a separately-managed subtree (eg per-request route content) transcluded into
/// a document layout without being owned by it.
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

	/// The Portal-aware parent of `entity`: the first holder rendering it in
	/// place (transcluded content's visual parent), else the [`ChildOf`]
	/// parent. The hop the style cascade inherits through; loop it to the top
	/// with [`Self::render_root`].
	pub fn visual_parent(
		parents: &Query<&ChildOf>,
		holders: &Query<&PortalOf>,
		entity: Entity,
	) -> Option<Entity> {
		holders
			.get(entity)
			.ok()
			.and_then(|portal_of| portal_of.holders().first().copied())
			.or_else(|| {
				parents.get(entity).ok().map(|child_of| child_of.parent())
			})
	}

	/// The render root of `entity`: the surface it renders on — the buffer host
	/// named by the nearest [`RenderSurface`] on its
	/// [`visual_parent`](Self::visual_parent) chain — else the top of that chain
	/// when it renders on no surface.
	///
	/// Scopes lookups (eg id resolution) to the tree an entity actually renders
	/// in, so concurrent surfaces (one per SSH session) never cross wires. A
	/// surface is a *visual* root even when it hangs under a shared owner by
	/// `ChildOf` (an SSH connection surface is a child of its router), so the
	/// walk must stop at the surface, never crossing up into that shared owner
	/// (whose subtree holds every other session's tree).
	pub fn render_root(
		parents: &Query<&ChildOf>,
		holders: &Query<&PortalOf>,
		surfaces: &Query<&RenderSurface>,
		entity: Entity,
	) -> Entity {
		let mut current = entity;
		loop {
			// a render surface is the visual root: resolve the host it renders into
			// and stop, so id resolution stays within this session's own tree.
			if let Ok(surface) = surfaces.get(current) {
				return surface.surface();
			}
			match Self::visual_parent(parents, holders, current) {
				// a self-referential edge would loop; a malformed graph is a clean stop.
				Some(parent) if parent != current => current = parent,
				_ => return current,
			}
		}
	}
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn walker_renders_referenced_entity() {
		let mut world = World::new();

		// content entity: <em>transcluded</em>
		let content = world.spawn(Element::new("em")).id();
		world.spawn((Value::Str("transcluded".into()), ChildOf(content)));

		// holder: a transparent entity that points at the content
		let root = world.spawn(Portal::new(content)).id();

		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
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

	/// Multi-tenant regression: [`render_root`](Portal::render_root) stops at the
	/// render surface, not at a shared owner the surface hangs under. Two session
	/// surfaces are `ChildOf` a common owner (as SSH connection surfaces are
	/// children of their router); a control in one session must resolve to *its*
	/// surface, so id-scoped lookups never reach the owner's subtree (which holds
	/// the other session's tree). Before the fix the walk crossed `ChildOf` past
	/// the surface up into the owner, so one session's disclosure toggled another
	/// session's target.
	#[beet_core::test]
	fn render_root_stops_at_the_surface() {
		let mut world = World::new();
		// the shared owner, eg the router the two SSH connections hang off.
		let owner = world.spawn_empty().id();
		// build a session: a buffer-host surface that is a ChildOf child of the
		// shared owner, its page transcluded into it by a Portal holder and
		// carrying `RenderSurface(host)`, exactly as the live page binding wires it.
		let session = |world: &mut World| -> (Entity, Entity) {
			let host = world.spawn(ChildOf(owner)).id();
			// the page carries `RenderSurface(host)` and holds the control; a holder
			// child of the host transcludes the page into the surface by `Portal`.
			let page = world.spawn(RenderSurface(host)).id();
			let control = world.spawn(ChildOf(page)).id();
			world.spawn((ChildOf(host), Portal::new(page)));
			(host, control)
		};
		let (host_a, control_a) = session(&mut world);
		let (host_b, _control_b) = session(&mut world);
		// A's control resolves to A's surface, never the shared owner or B's.
		world
			.with_state::<(
				Query<&ChildOf>,
				Query<&PortalOf>,
				Query<&RenderSurface>,
			), _>(move |(parents, holders, surfaces)| {
				Portal::render_root(&parents, &holders, &surfaces, control_a)
			})
			.xpect_eq(host_a);
		(host_a != host_b).xpect_true();
	}
}
