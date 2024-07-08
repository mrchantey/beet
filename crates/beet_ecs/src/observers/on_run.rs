use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, Event)]
pub struct OnRun;

#[derive(Debug, Default, Clone, Event, PartialEq, Deref)]
pub struct OnRunResult(RunResult);
impl OnRunResult {
	pub fn new(result: RunResult) -> Self { Self(result) }
	pub fn success() -> Self { Self(RunResult::Success) }
	pub fn failure() -> Self { Self(RunResult::Failure) }
	pub fn result(&self) -> RunResult { **self }
}

#[derive(Event)]
pub struct OnChildResult {
	child: Entity,
	result: RunResult,
}
impl OnChildResult {
	pub fn new(child: Entity, result: RunResult) -> Self {
		Self { child, result }
	}
	pub fn success(child: Entity) -> Self {
		Self::new(child, RunResult::Success)
	}
	pub fn failure(child: Entity) -> Self {
		Self::new(child, RunResult::Failure)
	}
	pub fn result(&self) -> RunResult { self.result }
	pub fn child(&self) -> Entity { self.child }
}
