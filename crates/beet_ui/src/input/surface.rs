//! The surface an interactive subtree is displayed on, the key that scopes input
//! (focus, scroll) to one session when many coexist in one world.

use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// Resolves which [`RenderSurface`] a (possibly deep) element belongs to: the
/// nearest self-or-ancestor carrying a [`RenderSurface`], walking `ChildOf`.
///
/// The single per-surface resolver. Every input system (focus, typing, tab,
/// form submit) and the terminal-title decoration share it instead of
/// hand-rolling the ancestor walk, so the scoping rule lives in one place.
/// Wraps the generalized [`AncestorQuery`] rather than re-walking `ChildOf`.
#[derive(SystemParam)]
pub struct SurfaceQuery<'w, 's> {
	ancestors: AncestorQuery<'w, 's, &'static RenderSurface>,
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
		self.ancestors.get(entity).ok().map(RenderSurface::surface)
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
