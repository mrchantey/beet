use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;

#[action(system=despawn_steer_target)]
#[derive(Default)]
pub struct DespawnSteerTarget;

fn despawn_steer_target(
	mut commands: Commands,
	agents: Query<&SteerTarget>,
	query: Query<&TargetAgent, (Added<Running>, With<DespawnSteerTarget>)>,
) {
	for agent in query.iter() {
		if let Ok(steer_target) = agents.get(**agent) {
			if let SteerTarget::Entity(target) = steer_target {
				commands.entity(*target).despawn();
			}
		}
	}
}
