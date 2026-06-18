//! The surface an interactive subtree is displayed on, the key that scopes input
//! (focus, scroll) to one session when many coexist in one world.

use beet_core::prelude::*;

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = RenderSurfaceOf)]
pub struct RenderSurface(#[entities] pub Entity);

impl RenderSurface {
	/// The surface entity this subtree is displayed on.
	pub fn surface(&self) -> Entity { self.0 }
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

/// The surface a (possibly deep) element belongs to: the nearest self-or-ancestor
/// carrying a [`RenderSurface`], walking `ChildOf`.
///
/// `None` for an element outside any surface. The app path resolves a surface for
/// every interactive element (the live page root carries one), so a `None` here is
/// a bare focusable with no page tree, which the per-surface input systems treat as
/// belonging to no surface (it receives no scoped input).
pub fn surface_of(
	entity: Entity,
	parents: &Query<&ChildOf>,
	surfaces: &Query<&RenderSurface>,
) -> Option<Entity> {
	let mut current = entity;
	loop {
		if let Ok(surface) = surfaces.get(current) {
			return Some(surface.0);
		}
		match parents.get(current) {
			Ok(child_of) => current = child_of.parent(),
			Err(_) => return None,
		}
	}
}

/// Whether an element on `element_surface` should receive input from `window`: the
/// element's surface must resolve and match. An element with no surface receives no
/// scoped input (fail-closed, so an unscoped element never leaks into a session).
pub fn surface_matches(
	element_surface: Option<Entity>,
	window: Entity,
) -> bool {
	element_surface == Some(window)
}
