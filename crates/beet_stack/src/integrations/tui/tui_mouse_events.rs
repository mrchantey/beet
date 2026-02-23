//! Mouse event types for the TUI input layer.
//!
//! These [`EntityEvent`] types are triggered by the input system when
//! the mouse interacts with terminal cells mapped to entities via
//! [`TuiSpanMap`](super::TuiSpanMap).
use beet_core::prelude::*;

/// Triggered when a mouse button is pressed over an entity.
#[derive(Debug, EntityEvent)]
pub struct TuiMouseDown {
	#[event_target]
	pub target: Entity,
}

/// Triggered when a mouse button is released over an entity.
#[derive(Debug, EntityEvent)]
pub struct TuiMouseUp {
	#[event_target]
	pub target: Entity,
}

/// Triggered when the mouse cursor enters an entity's region.
///
/// Only fires once per hover, not every frame.
#[derive(Debug, EntityEvent)]
pub struct TuiMouseOver {
	#[event_target]
	pub target: Entity,
}

/// Triggered when the mouse cursor leaves an entity's region.
///
/// Fires on the entity that was previously hovered when the cursor
/// moves to a different entity or to empty space.
#[derive(Debug, EntityEvent)]
pub struct TuiMouseOut {
	#[event_target]
	pub target: Entity,
}

/// Tracks which entity the mouse is currently hovering over.
///
/// Used by the input system to detect hover transitions and fire
/// [`TuiMouseOver`] / [`TuiMouseOut`] events.
#[derive(Debug, Default, Resource)]
pub struct TuiHoverState {
	/// The entity the mouse was over last frame, if any.
	pub hovered: Option<Entity>,
}
