//! # Utility AI - Score-Based Decision Making
//!
//! This example demonstrates utility AI, where the best action is selected
//! based on scores rather than fixed branching logic.
//!
//! ## Key Concepts
//!
//! - **HighestScore**: Selects the child with the highest score to run
//! - **Score**: A value (typically 0.0-1.0) representing action desirability
//! - **EndWith(Score)**: Returns a score instead of Pass/Fail
//!
//! ## How It Works
//!
//! 1. `HighestScore` triggers `GetScore` on all children
//! 2. Each child responds with its score via `EndWith(Score(value))`
//! 3. The child with the highest score is selected and triggered with `GetOutcome`
//! 4. That child's outcome becomes the parent's outcome
//!
//! ## Real-World Usage
//!
//! In practice, scores are computed dynamically based on game state:
//! - Distance to target
//! - Current health/resources
//! - Threat level
//! - Time since last action
//!
//! See `malenia.rs` for an example with custom score providers.
//!
//! ## Output
//!
//! ```text
//! OnRun: ScoreFlow will select the highest score
//! OnRun: this child runs
//! ```
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((ControlFlowPlugin::default(), DebugFlowPlugin::default()))
		.world_mut()
		.spawn((
			Name::new("ScoreFlow will select the highest score"),
			HighestScore::default(),
			children![
				(
					Name::new("this child does not run"),
					// Lower score - won't be selected
					EndWith(Score(0.4)),
				),
				(
					Name::new("this child runs"),
					// Higher score - will be selected
					EndWith(Score(0.6)),
				)
			],
		))
		.trigger_target(GetOutcome)
		.flush();
}
