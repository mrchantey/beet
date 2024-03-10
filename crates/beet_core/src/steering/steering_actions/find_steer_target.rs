use beet_ecs::prelude::*;
use bevy_core::prelude::*;
use bevy_math::Vec3;
use bevy_transform::components::Transform;

#[action(system=find_steer_target)]
#[derive(Default)]
pub struct FindSteerTarget {
	pub name: String,
}

// TODO this shouldnt run every frame?

fn find_steer_target(
	mut commands: Commands,
	agents: Query<&Transform>,
	names: Query<(Entity, &Transform, &Name)>,
	query: Query<(&TargetAgent, &FindSteerTarget), With<Running>>,
) {
	for (agent_entity, find_target) in query.iter() {
		if let Ok(agent_transform) = agents.get(**agent_entity) {
			let mut closest_dist = f32::MAX;
			let mut closest_target = None;

			for (target_entity, target_transform, name) in names.iter() {
				if **name == find_target.name {
					let new_dist = Vec3::distance(
						agent_transform.translation,
						target_transform.translation,
					);
					if new_dist < closest_dist {
						closest_dist = new_dist;
						closest_target = Some(target_entity);
					}
				}
			}

			if let Some(winner) = closest_target {
				commands
					.entity(**agent_entity)
					.insert(SteerTarget::Entity(winner));
			}
		}
	}
}
