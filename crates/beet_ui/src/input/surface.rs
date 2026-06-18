//! The surface an interactive subtree is displayed on, the key that scopes input
//! (focus, scroll) to one session when many coexist in one world.

use beet_core::prelude::*;

/// Back-link from a rendered subtree to the surface (the terminal/window entity)
/// it is displayed on.
///
/// Set on a subtree root (eg a live page root) so the per-surface input systems
/// resolve which surface a deep element belongs to by walking up to the nearest
/// ancestor carrying this. Input events carry their source `window`, so matching
/// an element's surface to an event's window scopes focus and typing per session,
/// letting many surfaces (one per SSH connection) coexist in one world.
#[derive(Debug, Clone, Copy, Component)]
pub struct RenderSurface(pub Entity);

/// The surface a (possibly deep) element belongs to: the nearest self-or-ancestor
/// carrying a [`RenderSurface`], walking `ChildOf`.
///
/// `None` for an element outside any surface (a single-surface app that never set
/// one), which the input systems treat as belonging to every surface, so a plain
/// single-window app needs no surface wiring.
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

/// Whether an element on `element_surface` should receive input from `window`: an
/// unscoped element (`None`) matches every window, else the surfaces must match.
pub fn surface_matches(element_surface: Option<Entity>, window: Entity) -> bool {
	element_surface.is_none_or(|surface| surface == window)
}
