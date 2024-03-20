use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive_action(Default)]
pub struct SucceedOnArrive {
	/// When the distance between the agent and the target is <= than this value, the action will succeed.
	pub radius: f32,
}

impl Default for SucceedOnArrive {
	fn default() -> Self { Self { radius: 0.5 } }
}

pub fn succeed_on_arrive(
	mut commands: Commands,
	agents: Query<(&Transform, &SteerTarget)>,
	transforms: Query<&Transform>,
	mut query: Query<(Entity, &TargetAgent, &SucceedOnArrive), With<Running>>,
) {
	for (entity, agent, succeed_on_arrive) in query.iter_mut() {
		if let Ok((transform, target)) = agents.get(**agent) {
			if let Ok(target) = target.position(&transforms) {
				if Vec3::distance(transform.translation, target)
					<= succeed_on_arrive.radius
				{
					commands.entity(entity).insert(RunResult::Success);
				}
			} else {
				commands.entity(entity).insert(RunResult::Failure);
			}
		}
	}
}
