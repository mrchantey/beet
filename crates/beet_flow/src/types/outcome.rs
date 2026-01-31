//! Declaration for the default [`RunEvent`] and [`EndEvent`] payloads: [`GetOutcome`] and [`Outcome`].
use crate::prelude::*;
use beet_core::prelude::*;


/// Requests an action to execute and return an [`Outcome`].
///
/// This is the most common [`RunEvent`], used to trigger behavior tree nodes
/// and other control flow actions. The action should eventually respond by
/// triggering an [`Outcome`] on itself.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// world
///     .spawn(EndWith(Outcome::Pass))
///     .trigger_target(GetOutcome);
/// ```
#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Reflect,
	EntityTargetEvent,
)]
pub struct GetOutcome;

impl RunEvent for GetOutcome {
	type End = Outcome;
}

/// The result of an action execution, indicating success or failure.
///
/// `Outcome` is conceptually similar to [`ControlFlow`] rather than [`Result`].
/// An [`Outcome::Fail`] status is frequently expected and does not necessarily
/// indicate an error.
///
/// For example, an `IsNearEnemy` action may emit `Outcome::Fail` to indicate
/// "I successfully checked and no, I am not near an enemy." For actual error
/// handling, the system/observer should return a [`Result`].
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// fn my_action(ev: On<GetOutcome>, mut commands: Commands) {
///     // Do some work...
///     let success = true;
///
///     // Return the outcome
///     let outcome = if success { Outcome::Pass } else { Outcome::Fail };
///     commands.entity(ev.target()).trigger_target(outcome);
/// }
/// ```
///
/// [`ControlFlow`]: std::ops::ControlFlow
#[derive(
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Reflect,
	EntityTargetEvent,
)]
pub enum Outcome {
	/// The action completed successfully or a predicate evaluated to true.
	///
	/// Similar to [`ControlFlow::Continue`](std::ops::ControlFlow::Continue).
	Pass,
	/// The action could not complete or a predicate evaluated to false.
	///
	/// This indicates an *expected* condition, unlike [`Err`] which signals
	/// an invalid program state.
	///
	/// Similar to [`ControlFlow::Break`](std::ops::ControlFlow::Break).
	Fail,
}
impl EndEvent for Outcome {
	type Run = GetOutcome;
}

impl Outcome {
	/// Returns `true` if the outcome is [`Outcome::Pass`].
	pub fn is_pass(&self) -> bool { self == &Outcome::Pass }
	/// Returns `true` if the outcome is [`Outcome::Fail`].
	pub fn is_fail(&self) -> bool { self == &Outcome::Fail }
}


impl std::fmt::Display for Outcome {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Outcome::Pass => write!(f, "Pass"),
			Outcome::Fail => write!(f, "Fail"),
		}
	}
}
