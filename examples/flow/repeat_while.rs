//! # Repeat While - Conditional Looping
//!
//! This example demonstrates how to implement "repeat while condition" patterns
//! using beet's `Repeat` action with a predicate child.
//!
//! ## Key Concepts
//!
//! - **Repeat::if_success()**: Repeats the sequence while children return Pass
//! - **SucceedTimes**: A predicate that passes N times, then fails
//! - **Sequence + Repeat**: Combines to create conditional loops
//!
//! ## How It Works
//!
//! 1. The root `Sequence` with `Repeat::if_success()` runs its children
//! 2. First child (`SucceedTimes(2)`) acts as the "while condition"
//! 3. If condition passes, second child runs (the actual work)
//! 4. Sequence restarts due to `Repeat::if_success()`
//! 5. After 2 iterations, `SucceedTimes` fails, breaking the loop
//!
//! ## Pattern Structure
//!
//! ```text
//! Sequence + Repeat::if_success()
//! ├── Condition (SucceedTimes) - controls loop continuation
//! └── Action (EndWith) - the work to perform each iteration
//! ```
//!
//! ## Output
//!
//! ```text
//! OnRun: root
//! OnRun: fails on third run
//! OnRun: some action to perform
//! OnRun: root                     (repeat #1)
//! OnRun: fails on third run
//! OnRun: some action to perform
//! OnRun: root                     (repeat #2)
//! OnRun: fails on third run       (fails here, loop exits)
//! done, subsequent updates will have no effect
//! ```
use beet::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		ControlFlowPlugin::default(),
		DebugFlowPlugin::default(),
	))
	.world_mut()
	.spawn((
		Name::new("root"),
		Sequence,
		// Repeat while children return Pass
		Repeat::if_success(),
		children![
			(
				Name::new("fails on third run"),
				// Acts as a "while condition" - passes twice, then fails
				SucceedTimes::new(2),
			),
			(
				Name::new("some action to perform"),
				// This is the actual work - runs each iteration until condition fails
				EndWith(Outcome::Pass),
			)
		],
	))
	.trigger_target(GetOutcome);
	app.update();
	app.update();
	println!("done, subsequent updates will have no effect");
	app.update();
	app.update();
	app.update();
}
