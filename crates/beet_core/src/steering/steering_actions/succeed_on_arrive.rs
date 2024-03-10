use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_transform::components::Transform;
use serde::Deserialize;
use serde::Serialize;

#[action(system=succeed_on_arrive)]
pub struct SucceedOnArrive {
	/// When the distance between the agent and the target is <= than this value, the action will succeed.
	pub radius: f32,
}

impl Default for SucceedOnArrive {
	fn default() -> Self { Self { radius: 0.5 } }
}

pub fn succeed_on_arrive(
	mut commands: Commands,
	transforms: Query<&Transform>,
	mut query: Query<
		(Entity, &Transform, &SteerTarget, &SucceedOnArrive),
		With<Running>,
	>,
) {
	for (entity, transform, target, succeed_on_arrive) in query.iter_mut() {
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
