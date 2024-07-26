use crate::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub mod child_expect {
	pub const NO_CHILDREN: &str = 
		"OnChildResult triggered but no children found";
	pub const NOT_MY_CHILD: &str =
		"OnChildResult triggered but caller not in children";
}


#[derive(Debug, Default, Clone, Copy, Event, PartialEq, Deref, Reflect)]
#[reflect(Default)]
pub struct OnRunResult(RunResult);
impl OnRunResult {
	pub fn new(result: RunResult) -> Self { Self(result) }
	pub fn success() -> Self { Self(RunResult::Success) }
	pub fn failure() -> Self { Self(RunResult::Failure) }
	pub fn result(&self) -> RunResult { **self }
}

pub type OnChildResult = OnChildValue<RunResult>;
impl OnChildResult {
	pub fn success(child: Entity) -> Self {
		Self::new(child, RunResult::Success)
	}
	pub fn failure(child: Entity) -> Self {
		Self::new(child, RunResult::Failure)
	}
}
