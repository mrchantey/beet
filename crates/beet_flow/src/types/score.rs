//! Scoring types for Utility AI decision-making.
//!
//! This module provides the [`GetScore`] and [`Score`] event pair used by
//! utility AI actions like [`HighestScore`] to evaluate and select children
//! based on their scores.
use crate::prelude::*;
use beet_core::prelude::*;


/// Requests an action to evaluate and return a [`Score`].
///
/// This is used by utility AI patterns where actions are selected based on
/// their computed score. For example, [`HighestScore`] triggers this event
/// on all children and runs the one with the highest score.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// world
///     .spawn(EndWith(Score::PASS))
///     .trigger_target(GetScore);
/// ```
#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	PartialEq,
	PartialOrd,
	Component,
	Reflect,
	EntityTargetEvent,
)]
pub struct GetScore;

impl RunEvent for GetScore {
	type End = Score;
}


/// A score value between 0.0 and 1.0 for utility AI evaluation.
///
/// Scores are used by actions like [`HighestScore`] to determine which child
/// action to execute. By convention, scores should be normalized to the range
/// `[0.0, 1.0]` where:
/// - `0.0` ([`Score::FAIL`]) indicates lowest priority
/// - `0.5` ([`Score::NEUTRAL`]) indicates neutral priority
/// - `1.0` ([`Score::PASS`]) indicates highest priority
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// // Create an action that always scores 0.75
/// world.spawn(EndWith(Score(0.75)));
/// ```
#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	PartialEq,
	PartialOrd,
	Deref,
	DerefMut,
	Component,
	Reflect,
	EntityTargetEvent,
)]
pub struct Score(pub f32);

impl EndEvent for Score {
	type Run = GetScore;
}

impl Score {
	/// The maximum score value (1.0), indicating highest priority.
	pub const PASS: Self = Self(1.0);
	/// A neutral score value (0.5).
	pub const NEUTRAL: Self = Self(0.5);
	/// The minimum score value (0.0), indicating lowest priority.
	pub const FAIL: Self = Self(0.0);

	/// Creates a new [`Score`] with the given value.
	pub fn new(score: f32) -> Self { Self(score) }
}
