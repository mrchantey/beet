//! Renderer-agnostic pointer event types for entity interaction.
//!
//! These [`EntityEvent`] types are triggered by renderer-specific
//! input systems when a pointer interacts with entities. A pointer is a
//! general interaction device: in XR it could be a hit test or finger
//! collision, in a TUI it could be a mouse cursor or keyboard-driven
//! cursor.
//!
//! The renderer resolves screen positions to entities and fires these
//! generic events via [`trigger`](bevy::ecs::world::EntityWorldMut::trigger),
//! each carrying the [`Entity`] of the pointer that triggered the
//! interaction.
use crate::prelude::ElementState;
use crate::prelude::ElementStateMap;
use beet_core::prelude::*;

/// Keeps each element's [`ElementStateMap`] carrying
/// [`Hovered`](ElementState::Hovered) while a pointer is over it, so `:hover`
/// rules apply. [`PointerOver`]/[`PointerOut`] auto-propagate, so an element
/// hovers whenever the pointer is over any of its descendants, like CSS.
///
/// Renderer-agnostic: whichever input system fires the pointer events drives
/// this (the charcell hit-test today).
#[derive(Default)]
pub struct PointerStatePlugin;

impl Plugin for PointerStatePlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(hover_state_on_over)
			.add_observer(hover_state_on_out);
	}
}

/// Observer: mark each element in the [`PointerOver`] propagation chain hovered.
fn hover_state_on_over(
	ev: On<PointerOver>,
	elements: Query<(), With<Element>>,
	mut states: Query<&mut ElementStateMap>,
	mut commands: Commands,
) {
	let target = ev.event_target();
	if !elements.contains(target) {
		return;
	}
	if let Ok(mut map) = states.get_mut(target) {
		if !map.contains(&ElementState::Hovered) {
			map.insert(ElementState::Hovered);
		}
	} else {
		commands
			.entity(target)
			.insert(ElementStateMap::with(ElementState::Hovered));
	}
}

/// Observer: clear the hovered state from each element the pointer left.
fn hover_state_on_out(
	ev: On<PointerOut>,
	mut states: Query<&mut ElementStateMap>,
) {
	if let Ok(mut map) = states.get_mut(ev.event_target()) {
		if map.contains(&ElementState::Hovered) {
			map.remove(&ElementState::Hovered);
		}
	}
}

/// Tracks which entity a pointer is currently hovering over.
///
/// One pointer lives on each interactive surface (the terminal/window entity),
/// so many surfaces (one per SSH session, or two XR hands) independently track
/// their own hover target. Input events carry their source `window`, so each is
/// routed to that surface's pointer.
#[derive(Debug, Default, Clone, Reflect, Component)]
#[reflect(Component)]
pub struct Pointer {
	/// The entity the pointer was over last frame, if any.
	pub hover: Option<Entity>,
}

/// Triggered when a pointer button is pressed over an entity.
///
/// In a TUI this corresponds to a mouse button press; in XR it
/// could be a close-pinch gesture.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct PointerDown {
	/// The entity this event is targeting.
	#[event_target]
	pub target: Entity,
	/// The pointer that triggered this event.
	pub pointer: Entity,
}

impl PointerDown {
	/// Constructs the event for the given `pointer`, deferring the target.
	pub fn new(pointer: Entity) -> impl FnOnce(Entity) -> Self {
		move |target| Self { target, pointer }
	}
}

/// Triggered when a pointer button is released over an entity.
///
/// In a TUI this corresponds to a mouse button release; in XR it
/// could be an open-pinch gesture.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct PointerUp {
	/// The entity this event is targeting.
	#[event_target]
	pub target: Entity,
	/// The pointer that triggered this event.
	pub pointer: Entity,
}

impl PointerUp {
	/// Constructs the event for the given `pointer`, deferring the target.
	pub fn new(pointer: Entity) -> impl FnOnce(Entity) -> Self {
		move |target| Self { target, pointer }
	}
}

/// Triggered when a pointer enters an entity's region.
///
/// Only fires once per hover, not every frame.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct PointerOver {
	/// The entity this event is targeting.
	#[event_target]
	pub target: Entity,
	/// The pointer that triggered this event.
	pub pointer: Entity,
}

impl PointerOver {
	/// Constructs the event for the given `pointer`, deferring the target.
	pub fn new(pointer: Entity) -> impl FnOnce(Entity) -> Self {
		move |target| Self { target, pointer }
	}
}

/// Triggered when a pointer leaves an entity's region.
///
/// Fires on the entity that was previously hovered when the pointer
/// moves to a different entity or to empty space.
#[derive(Debug, EntityEvent)]
#[entity_event(propagate, auto_propagate)]
pub struct PointerOut {
	/// The entity this event is targeting.
	#[event_target]
	pub target: Entity,
	/// The pointer that triggered this event.
	pub pointer: Entity,
}

impl PointerOut {
	/// Constructs the event for the given `pointer`, deferring the target.
	pub fn new(pointer: Entity) -> impl FnOnce(Entity) -> Self {
		move |target| Self { target, pointer }
	}
}
