//! The surface an interactive subtree is displayed on, the key that scopes input
//! (focus, scroll) to one session when many coexist in one world.

use crate::prelude::RenderTreeQuery;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// Resolves which [`RenderSurface`] a (possibly deep) element belongs to: the
/// nearest self-or-ancestor carrying a [`RenderSurface`], walking *visual*
/// ancestry through [`RenderTreeQuery`], so each
/// [`Portal`](crate::prelude::Portal) transclusion is crossed to its holder.
///
/// The single per-surface resolver. Every input system (focus, typing, tab,
/// form submit) and the terminal-title decoration share it instead of
/// hand-rolling the ancestor walk, so the scoping rule lives in one place.
///
/// Crossing the portal is what lets transcluded route content resolve its
/// surface: the live page layouts the route content into a `<Slot>` by reference
/// (a `Portal`), so a link/field deep in the content has no `ChildOf` path to the
/// page root's `RenderSurface`. Following the holder bridges that gap, exactly as
/// the scroll input does for its scroll-container walk.
#[derive(SystemParam)]
pub struct SurfaceQuery<'w, 's> {
	surfaces: Query<'w, 's, &'static RenderSurface>,
	tree: RenderTreeQuery<'w, 's>,
}

impl SurfaceQuery<'_, '_> {
	/// The surface entity `entity` is displayed on, or `None` if it sits outside
	/// any surface.
	///
	/// The app path resolves a surface for every interactive element (the live
	/// page root carries one), so a `None` here is a bare focusable with no page
	/// tree, which the per-surface input systems treat as belonging to no surface
	/// (it receives no scoped input).
	pub fn surface_of(&self, entity: Entity) -> Option<Entity> {
		self.tree
			.iter_ancestors_inclusive(entity)
			.find_map(|ancestor| {
				self.surfaces.get(ancestor).ok().map(RenderSurface::surface)
			})
	}

	/// The render root of `entity`: the [surface](Self::surface_of) it renders
	/// on, else the top of its visual ancestry when it renders on no surface.
	///
	/// Scopes lookups (eg id resolution) to the tree an entity actually renders
	/// in, so concurrent surfaces (one per SSH session) never cross wires. A
	/// surface is a *visual* root even when it hangs under a shared owner by
	/// `ChildOf` (an SSH connection surface is a child of its router), so the
	/// walk stops at the surface, never crossing up into that shared owner
	/// (whose subtree holds every other session's tree).
	pub fn render_root(&self, entity: Entity) -> Entity {
		self.surface_of(entity).unwrap_or_else(|| {
			self.tree
				.iter_ancestors_inclusive(entity)
				.last()
				.unwrap_or(entity)
		})
	}

	/// Whether `entity` should receive input sourced from `window`: its surface
	/// must resolve and equal `window`.
	///
	/// Fail-closed, so an unscoped element (no surface) never leaks into a
	/// session.
	pub fn matches(&self, entity: Entity, window: Entity) -> bool {
		self.surface_of(entity) == Some(window)
	}
}

/// The surface (the terminal/window entity) a rendered subtree is displayed on,
/// the source half of the one-to-one [`RenderSurfaceOf`] relationship.
///
/// Set on a subtree root (eg a live page root) so the per-surface input systems
/// resolve which surface a deep element belongs to by walking up to the nearest
/// ancestor carrying this. Input events carry their source `window`, so matching
/// an element's surface to an event's window scopes focus and typing per session,
/// letting many surfaces (one per SSH connection) coexist in one world.
///
/// One page per surface: binding a new page's `RenderSurface` to a host replaces
/// the previous page's, so the host's [`RenderSurfaceOf`] always names its single
/// current page (like [`Portal`](crate::prelude::Portal) but one-to-one).
///
/// `allow_self_referential` so a host that *is* its own surface (eg a directly
/// spawned charcell chat host that skips the router's page binding) can carry
/// `RenderSurface(self)`, scoping its whole subtree to itself in one component.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = RenderSurfaceOf, allow_self_referential)]
pub struct RenderSurface(#[entities] pub Entity);

impl RenderSurface {
	/// The surface entity this subtree is displayed on.
	pub fn surface(&self) -> Entity { self.0 }

	/// An [`OnSpawn`] that makes the entity its own surface: it inserts
	/// `RenderSurface(self)`, so the whole subtree resolves to this entity through
	/// [`SurfaceQuery`] with no per-widget wiring. For a directly-spawned host that
	/// skips the router's page binding (eg a charcell chat host).
	pub fn self_referential() -> OnSpawn {
		OnSpawn::new(|entity: &mut EntityWorldMut| {
			let entity_id = entity.id();
			entity.insert(RenderSurface(entity_id));
		})
	}
}

/// The page currently bound to this surface, the target half of the
/// [`RenderSurface`] relationship: one entity, replaced when a new page binds.
///
/// Lets the surface name its current page directly (eg to despawn the outgoing
/// page when a navigation swaps it), without reading the render slot.
#[derive(Debug, Clone, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = RenderSurface)]
pub struct RenderSurfaceOf(Entity);

impl RenderSurfaceOf {
	/// The page currently displayed on this surface.
	pub fn page(&self) -> Entity { self.0 }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// `surface_of` resolves the surface for content transcluded by a [`Portal`]:
	/// the content has no `ChildOf` link to the page root, so the walk must cross
	/// the holder. This is the markdown-link regression — a route's content is
	/// layouted into a `<Slot>` by reference, so a link inside it could not reach
	/// the page root's [`RenderSurface`] and never navigated.
	#[beet_core::test]
	fn surface_resolves_across_portal() {
		let mut world = World::new();
		let window = world.spawn_empty().id();
		// the transcluded content: a link nested under a free root (no ChildOf to
		// the page), mirroring per-request route content.
		let link = world.spawn(Element::new("a")).id();
		let content = world.spawn_empty().add_child(link).id();
		// the page root carries the surface and transcludes the content into a slot
		// holder by reference (a Portal), exactly as the layout middleware does.
		world.spawn((RenderSurface(window), children![Portal::new(content)]));

		world
			.with_state::<SurfaceQuery, _>(|surfaces| surfaces.surface_of(link))
			.xpect_eq(Some(window));
	}

	/// Multi-tenant regression: [`render_root`](SurfaceQuery::render_root)
	/// stops at the render surface, not at a shared owner the surface hangs
	/// under. Two session surfaces are `ChildOf` a common owner (as SSH
	/// connection surfaces are children of their router); a control in one
	/// session must resolve to *its* surface, so id-scoped lookups never reach
	/// the owner's subtree (which holds the other session's tree). A walk
	/// crossing `ChildOf` past the surface up into the owner once let one
	/// session's disclosure toggle another session's target.
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
			let page = world.spawn(RenderSurface(host)).id();
			let control = world.spawn(ChildOf(page)).id();
			world.spawn((ChildOf(host), Portal::new(page)));
			(host, control)
		};
		let (host_a, control_a) = session(&mut world);
		let (host_b, _control_b) = session(&mut world);
		// A's control resolves to A's surface, never the shared owner or B's.
		world
			.with_state::<SurfaceQuery, _>(|surfaces| {
				surfaces.render_root(control_a)
			})
			.xpect_eq(host_a);
		(host_a != host_b).xpect_true();
	}

	/// A bare element outside any surface (no `RenderSurface` ancestor, no holder)
	/// resolves to `None` rather than looping, the fail-closed default.
	#[beet_core::test]
	fn no_surface_resolves_none() {
		let mut world = World::new();
		let orphan = world.spawn(Element::new("a")).id();
		world
			.with_state::<SurfaceQuery, _>(|surfaces| {
				surfaces.surface_of(orphan)
			})
			.xpect_eq(None);
	}
}
