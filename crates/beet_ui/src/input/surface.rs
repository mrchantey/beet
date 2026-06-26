//! The surface an interactive subtree is displayed on, the key that scopes input
//! (focus, scroll) to one session when many coexist in one world.

use crate::prelude::PortalOf;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// Resolves which [`RenderSurface`] a (possibly deep) element belongs to: the
/// nearest self-or-ancestor carrying a [`RenderSurface`], walking *visual*
/// ancestry — `ChildOf`, crossing each [`Portal`](crate::prelude::Portal)
/// transclusion via its [`PortalOf`] holder.
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
	parents: Query<'w, 's, &'static ChildOf>,
	portals: Query<'w, 's, &'static PortalOf>,
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
		let mut current = entity;
		loop {
			if let Ok(surface) = self.surfaces.get(current) {
				return Some(surface.surface());
			}
			// transclusion wins for visual ancestry: a Portal holder is the visual
			// parent of the content it renders in place, so cross it before walking
			// `ChildOf`. The two are mutually exclusive at a node in practice (a
			// transcluded root has a holder but no parent), so this just bridges the
			// gap where the `ChildOf` chain dead-ends at the content root.
			current = self
				.portals
				.get(current)
				.ok()
				.and_then(|portal_of| portal_of.holders().first().copied())
				.or_else(|| {
					self.parents.get(current).ok().map(ChildOf::parent)
				})?;
		}
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
