//! Renderer-agnostic pointer event types for entity interaction.
//!
//! These [`EntityTargetEvent`] types are triggered by renderer-specific
//! input systems when a pointer interacts with entities. A pointer is a
//! general interaction device: in XR it could be a hit test or finger
//! collision, in a TUI it could be a mouse cursor or keyboard-driven
//! cursor.
//!
//! The renderer resolves screen positions to entities and fires these
//! generic events via [`trigger_target`](beet_core::prelude::EntityWorldMutActionEventExt::trigger_target),
//! each carrying the [`Entity`] of the pointer that triggered the
//! interaction.
use beet_core::prelude::*;

/// Tracks which entity a pointer is currently hovering over.
///
/// Each pointer entity carries its own hover state so that
/// multiple pointers (eg two XR hands) can independently track
/// hover targets.
#[derive(Debug, Default, Clone, Reflect, Component)]
#[reflect(Component)]
pub struct Pointer {
	/// The entity the pointer was over last frame, if any.
	pub hover: Option<Entity>,
}

/// Marker for the primary pointer.
///
/// There should only ever be one entity with this component.
/// Global mouse/cursor events are routed through the primary
/// pointer, for example the TUI input system reads the hover
/// state from this entity.
#[derive(Debug, Default, Clone, Copy, Reflect, Component)]
#[reflect(Component)]
#[require(Pointer)]
pub struct PrimaryPointer;

/// Triggered when a pointer button is pressed over an entity.
///
/// In a TUI this corresponds to a mouse button press; in XR it
/// could be a close-pinch gesture.
#[derive(Debug, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct PointerDown {
	/// The pointer that triggered this event.
	pub pointer: Entity,
}

/// Triggered when a pointer button is released over an entity.
///
/// In a TUI this corresponds to a mouse button release; in XR it
/// could be an open-pinch gesture.
#[derive(Debug, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct PointerUp {
	/// The pointer that triggered this event.
	pub pointer: Entity,
}

/// Triggered when a pointer enters an entity's region.
///
/// Only fires once per hover, not every frame.
#[derive(Debug, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct PointerOver {
	/// The pointer that triggered this event.
	pub pointer: Entity,
}

/// Triggered when a pointer leaves an entity's region.
///
/// Fires on the entity that was previously hovered when the pointer
/// moves to a different entity or to empty space.
#[derive(Debug, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct PointerOut {
	/// The pointer that triggered this event.
	pub pointer: Entity,
}
