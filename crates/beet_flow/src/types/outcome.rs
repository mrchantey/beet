//! Declaration for the default [`Run`] & [`End`] payloads: [`GetOutcome`] and [`Outcome`]
use crate::prelude::*;
use beet_core::prelude::*;


/// The default [`Run`] payload, requesting the action to return an [`Outcome`]
#[derive(
	Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect,
)]
pub struct GetOutcome;

impl RunPayload for GetOutcome {
	type End = Outcome;
}
impl EventPayload for GetOutcome {
	type Event = Run<GetOutcome>;
	fn into_event(self, entity: Entity) -> Self::Event {
		Run::new(entity, self)
	}
}

/// The default [`End`] payload, used to indicate run status. [`Outcome`]
/// is closer in spirit to [`ControlFlow`] than [`Result`], in that the
/// [`Outcome::Fail`] status is frequently expected, and does not
/// nessecarily indicate an error.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum Outcome {
	/// The action emitted a `Pass` outcome, which may mean it was able to
	/// execute successfully or that a predicate passed.
	Pass,
	/// The action emitted a `Fail` outcome, this is not nessecarily an `Err`,
	/// for example an `IsNearEnemy` action may emit `Outcome::Fail` to indicate
	/// 'no i am not near an enemy'.
	Fail,
}
impl EndPayload for Outcome {
	type Run = GetOutcome;
}
impl EventPayload for Outcome {
	type Event = End<Outcome>;
	fn into_event(self, entity: Entity) -> Self::Event {
		End::new(entity, self)
	}
}


impl Outcome {
	pub fn is_pass(&self) -> bool { self == &Outcome::Pass }
	pub fn is_fail(&self) -> bool { self == &Outcome::Fail }
}
