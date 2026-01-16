//! # Hello World - Basic Behavior Tree
//!
//! This example demonstrates the fundamental concepts of beet's control flow system:
//!
//! - **Sequence**: Runs children in order until one fails or all succeed
//! - **GetOutcome**: The event that triggers an action to run
//! - **Outcome**: The result (Pass/Fail) that actions return
//! - **EndWith**: A simple action that immediately returns a specified outcome
//!
//! ## How It Works
//!
//! 1. We create a root entity with a `Sequence` component
//! 2. Two children are spawned, each with `EndWith(Outcome::Pass)`
//! 3. When `GetOutcome` is triggered on root, it runs child1, then child2
//! 4. Both pass, so the sequence completes successfully
//!
//! ## Output
//!
//! ```text
//! OnRun: root
//! OnRun: child1
//! OnRun: child2
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			ControlFlowPlugin::default(),
			// DebugFlowPlugin logs when actions run
			DebugFlowPlugin::default(),
		))
		.world_mut()
		.spawn((Name::new("root"), Sequence))
		.with_child((Name::new("child1"), EndWith(Outcome::Pass)))
		.with_child((Name::new("child2"), EndWith(Outcome::Pass)))
		// Trigger the behavior tree to run
		.trigger_target(GetOutcome)
		// Flush ensures all deferred commands execute
		.flush();
}
