use crate::prelude::*;
use bevy::prelude::*;

#[derive_action]
#[action(graph_role=GraphRole::World)]
/// Does what it says on the tin, useful for tests
pub struct EmptyAction;
pub fn empty_action() {}


#[derive_action]
#[action(graph_role=GraphRole::Node)]
/// Reattaches the [`Running`] component whenever it is removed.
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
