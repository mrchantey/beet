use crate::prelude::*;
use bevy::prelude::*;


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
