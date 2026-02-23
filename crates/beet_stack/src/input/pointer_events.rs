//! Renderer-agnostic event types for entity interaction.
//!
//! These [`EntityEvent`] types are triggered by renderer-specific input
//! systems when the 'pointer' interacts with entities. The renderer resolves
//! screen positions to entities and fires these generic events.
use beet_core::prelude::*;

/// Triggered when a mouse button is pressed over an entity.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct MouseDown {
	#[event_target]
	pub target: Entity,
}

/// Triggered when a mouse button is released over an entity.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct MouseUp {
	#[event_target]
	pub target: Entity,
}

/// Triggered when the mouse cursor enters an entity's region.
///
/// Only fires once per hover, not every frame.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct MouseOver {
	#[event_target]
	pub target: Entity,
}

/// Triggered when the mouse cursor leaves an entity's region.
///
/// Fires on the entity that was previously hovered when the cursor
/// moves to a different entity or to empty space.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct MouseOut {
	#[event_target]
	pub target: Entity,
}

/// Tracks which entity the mouse is currently hovering over.
///
/// Used by input systems to detect hover transitions and fire
/// [`MouseOver`] / [`MouseOut`] events.
#[derive(Debug, Default, Resource)]
pub struct HoverState {
	/// The entity the mouse was over last frame, if any.
	pub hovered: Option<Entity>,
}
