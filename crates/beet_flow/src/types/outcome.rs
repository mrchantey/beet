//! Declaration for the default [`Run`] & [`End`] payloads: [`GetOutcome`] and [`Outcome`]
use crate::prelude::*;
use beet_core::prelude::*;


/// The most common [`RunEvent`], request an action to return an [`Outcome`]
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
	ActionEvent,
)]
pub struct GetOutcome;

impl RunEvent for GetOutcome {
	type End = Outcome;
}

/// The returned value from a [`GetOutcome`] request, indicating run status.
/// [`Outcome`] is closer in spirit to [`ControlFlow`] than [`Result`], in that the
/// [`Outcome::Fail`] status is frequently expected, and does not
/// necessarily indicate an error.
/// For example an `IsNearEnemy` action may emit `Outcome::Fail` to indicate
/// 'I successfully checked and no i am not near an enemy'.
/// For actual error handling the system/observer should output a [`Result`]
#[derive(
	Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect, ActionEvent,
)]
pub enum Outcome {
	/// More like a [`ControlFlow::Continue`] than an [`Ok`], may mean this action
	/// was able to execute successfully or that a predicate passed.
	Pass,
	/// More like a [`ControlFlow::Break`] than an [`Err`], may mean a predicate failed
	/// or this action could not execute for an *expected reason*, unlike an [`Err`] which
	/// indicates some invalid state of the program.
	Fail,
}
impl EndEvent for Outcome {
	type Run = GetOutcome;
}

impl Outcome {
	/// Returns `true` if the outcome is [`Outcome::Pass`]
	pub fn is_pass(&self) -> bool { self == &Outcome::Pass }
	/// Returns `true` if the outcome is [`Outcome::Fail`]
	pub fn is_fail(&self) -> bool { self == &Outcome::Fail }
}
