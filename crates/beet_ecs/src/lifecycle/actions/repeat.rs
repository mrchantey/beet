use crate::prelude::*;
use bevy::prelude::*;


#[derive(Debug, Default, Clone, PartialEq, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(repeat.in_set(PreTickSet))]
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
