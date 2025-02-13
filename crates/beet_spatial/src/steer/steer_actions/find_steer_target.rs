use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Sets the [`SteerTarget`] when an entity with the given name is nearby.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct FindSteerTarget {
	pub name: String,
	// #[inspector(min = 0., max = 3., step = 0.1)]
	pub radius: f32,
}

impl Default for FindSteerTarget {
	fn default() -> Self {
		Self {
			name: "enemy".to_string(),
			radius: 10.0,
		}
	}
}

impl FindSteerTarget {
	pub fn new(name: impl Into<String>, radius: f32) -> Self {
		Self {
			name: name.into(),
			radius,
		}
	}
}

// TODO this shouldnt run every frame?

pub(crate) fn find_steer_target(
	mut commands: Commands,
	agents: Query<&Transform>,
	names: Query<(Entity, &Transform, &Name)>,
	query: Query<(&Running, &FindSteerTarget)>,
) {
	for (running, find_target) in query.iter() {
		if let Ok(agent_transform) = agents.get(running.origin) {
			let mut closest_dist = f32::MAX;
			let mut closest_target = None;

			for (entity, target_transform, name) in names.iter() {
				if **name == find_target.name {
					let new_dist = Vec3::distance(
						agent_transform.translation,
						target_transform.translation,
					);
					if new_dist <= find_target.radius && new_dist < closest_dist
					{
						closest_dist = new_dist;
						closest_target = Some(entity);
					}
				}
			}

			if let Some(winner) = closest_target {
				commands
					.entity(running.origin)
					.insert(SteerTarget::Entity(winner));
			}
		}
	}
}
