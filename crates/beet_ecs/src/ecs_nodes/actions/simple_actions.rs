use crate::prelude::*;
use bevy_ecs::prelude::*;

#[derive(Default)]
#[derive_action]
pub struct EmptyAction;
pub fn empty_action() {}


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
