//! # State Machine - Graph-Based Transitions
//!
//! This example shows how to implement state machine patterns in beet using `RunNext`.
//!
//! ## Key Concepts
//!
//! - **RunNext**: Triggers another behavior after the current one completes
//! - **State-like patterns**: Any entity can act as a "state"
//! - **Transitions**: Connect states via `RunNext` for explicit flow control
//!
//! ## How It Works
//!
//! Unlike behavior trees which use parent-child relationships, state machines
//! use arbitrary connections between entities:
//!
//! ```text
//! state1 ---> transition ---> state2
//! ```
//!
//! 1. `state1` runs and completes with `Outcome::Pass`
//! 2. `RunNext(transition)` triggers the transition entity
//! 3. `transition` runs and its `RunNext(state2)` triggers state2
//! 4. `state2` runs as the final state
//!
//! ## When to Use
//!
//! State machines are useful when:
//! - States can transition to multiple different states
//! - Transitions depend on runtime conditions
//! - You need explicit control over flow (vs. implicit tree traversal)
//!
//! ## Output
//!
//! ```text
//! OnRun: state1
//! OnRun: transition
//! OnRun: state2
//! ```
use beet::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins((ControlFlowPlugin::default(), DebugFlowPlugin::default()));
	let world = app.world_mut();

	// Create state2 first since state1 needs its entity ID
	let state2 = world
		.spawn((Name::new("state2"), EndWith(Outcome::Pass)))
		.id();

	// Transitions are behaviors that trigger the next state
	let transition = world
		.spawn((
			Name::new("transition"),
			EndWith(Outcome::Pass),
			RunNext::new(state2),
		))
		.id();

	// Initial state triggers the transition when it completes
	world
		.spawn((
			Name::new("state1"),
			EndWith(Outcome::Pass),
			// RunNext can be swapped with control flow logic
			// to decide which state to transition to
			RunNext::new(transition),
		))
		.trigger_target(GetOutcome)
		.flush();
}
