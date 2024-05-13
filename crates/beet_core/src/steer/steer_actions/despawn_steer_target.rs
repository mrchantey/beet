use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
/// Recursively despawns the [`SteerTarget`]
pub struct DespawnSteerTarget;

fn despawn_steer_target(
	mut commands: Commands,
	agents: Query<(Entity, &SteerTarget)>,
	query: Query<&TargetAgent, (With<Running>, With<DespawnSteerTarget>)>,
) {
	for target_agent in query.iter() {
		if let Ok((agent, steer_target)) = agents.get(**target_agent) {
			if let SteerTarget::Entity(target) = steer_target {
				if let Some(entity) = commands.get_entity(*target) {
					// this will occasionally error Entity not found
					entity.despawn_recursive();
					commands.entity(agent).remove::<SteerTarget>();
				}
			}
		}
	}
}

impl ActionMeta for DespawnSteerTarget {
	fn category(&self) -> ActionCategory { ActionCategory::World }
}

impl ActionSystems for DespawnSteerTarget {
	fn systems() -> SystemConfigs { despawn_steer_target.in_set(TickSet) }
}
