use crate::prelude::*;
use bevy_ecs::prelude::*;

#[derive(Default)]
#[derive_action]
pub struct EmptyAction;
pub fn empty_action() {}

// intentionally dont deref to avoid bugs.
// TODO this should be generic
#[derive_action]
#[derive(Default)]
pub struct SetRunResult(pub RunResult);

impl SetRunResult {
	pub fn new(result: RunResult) -> Self { Self(result) }
	pub fn success() -> Self { Self(RunResult::Success) }
	pub fn failure() -> Self { Self(RunResult::Failure) }
}

fn set_run_result(
	mut commands: Commands,
	query: Query<(Entity, &SetRunResult), With<Running>>,
) {
	for (entity, result) in query.iter() {
		commands.entity(entity).insert(result.0);
	}
}


#[derive_action]
#[derive(Default)]
pub struct Repeat;

/// This relys on [`sync_running`]
fn repeat(
	mut commands: Commands,
	mut query: Query<(Entity, &Repeat), (With<RunResult>, Without<Running>)>,
) {
	for (entity, _repeat) in query.iter_mut() {
		commands.entity(entity).insert(Running);
		log::info!("repeat");
	}
}
