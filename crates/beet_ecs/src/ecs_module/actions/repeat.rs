use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;


#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
/// Reattaches the [`Running`] component whenever it is removed.
pub struct Repeat;

impl ActionMeta for Repeat {
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

impl ActionSystems for Repeat {
	fn systems() -> SystemConfigs { repeat.in_set(PreTickSet) }
}

/// This relys on [`sync_running`]
fn repeat(
	mut commands: Commands,
	mut query: Query<(Entity, &Repeat), (With<RunResult>, Without<Running>)>,
) {
	for (entity, _repeat) in query.iter_mut() {
		commands.entity(entity).insert(Running);
	}
}