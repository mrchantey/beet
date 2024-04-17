use crate::prelude::*;
use bevy::prelude::*;

#[derive_action]
#[action(graph_role=GraphRole::World)]
pub struct EmptyAction;
pub fn empty_action() {}


#[derive_action]
#[action(graph_role=GraphRole::Node)]
pub struct Repeat;

/// This relys on [`sync_running`]
fn repeat(
	mut commands: Commands,
	mut query: Query<(Entity, &Repeat), (With<RunResult>, Without<Running>)>,
) {
	for (entity, _repeat) in query.iter_mut() {
		commands.entity(entity).insert(Running);
	}
}
