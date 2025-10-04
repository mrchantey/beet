//! Declaration for the default [`Run`] & [`End`] payloads: [`GetOutcome`] and [`Outcome`]
use crate::prelude::*;
use beet_core::prelude::*;


/// The default [`Run`] payload, requesting the action to return an [`Outcome`]
#[derive(
	Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect,
)]
pub struct GetOutcome;

/// Type alias for the default [`Run`] payload: [`GetOutcome`]
pub const RUN: GetOutcome = GetOutcome;
/// Type alias for the default [`End`] payload: [`Outcome::Success`]
pub const SUCCESS: Outcome = Outcome::Success;
/// Type alias for the default [`End`] payload: [`Outcome::Failure`]
pub const FAILURE: Outcome = Outcome::Failure;

impl RunPayload for GetOutcome {
	type End = Outcome;
}
impl EventPayload for GetOutcome {
	type Event = Run<GetOutcome>;
	fn into_event(self, entity: Entity) -> Self::Event {
		Run::new(entity, self)
	}
}

/// The most common End payload, used to indicate run status
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Reflect)]
pub enum Outcome {
	Success,
	Failure,
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
	pub fn is_success(&self) -> bool { self == &Outcome::Success }
	pub fn is_failure(&self) -> bool { self == &Outcome::Failure }
}
