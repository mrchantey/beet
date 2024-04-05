use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive_action]
#[action(graph_role=GraphRole::Other)]
pub struct DespawnSteerTarget;

fn despawn_steer_target(
	mut commands: Commands,
	agents: Query<(Entity, &SteerTarget)>,
	query: Query<&ParentRoot, (With<Running>, With<DespawnSteerTarget>)>,
) {
	for target_agent in query.iter() {
		if let Ok((agent, steer_target)) = agents.get(**target_agent) {
			if let SteerTarget::Entity(target) = steer_target {
				if let Some(mut entity) = commands.get_entity(*target) {
					// this will occasionally error Entity not found
					entity.despawn();
					commands.entity(agent).remove::<SteerTarget>();
				}
			}
		}
	}
}
