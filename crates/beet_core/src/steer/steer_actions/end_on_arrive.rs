use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


/// Succeeds when the agent arrives at the [`SteerTarget`].
/// Fails if the target is not found.
#[derive(Debug, Clone, PartialEq, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(succeed_on_arrive.in_set(TickSet))]
pub struct EndOnArrive {
	pub radius: f32,
}

impl Default for EndOnArrive {
	fn default() -> Self { Self { radius: 0.5 } }
}

impl EndOnArrive {
	pub fn new(radius: f32) -> Self { Self { radius } }
}

pub fn succeed_on_arrive(
	mut commands: Commands,
	agents: Query<(&Transform, &SteerTarget)>,
	transforms: Query<&Transform>,
	mut query: Query<(Entity, &TargetAgent, &EndOnArrive), With<Running>>,
) {
	for (entity, agent, succeed_on_arrive) in query.iter_mut() {
		if let Ok((transform, target)) = agents.get(**agent) {
			if let Ok(target) = target.position(&transforms) {
				if Vec3::distance(transform.translation, target)
					<= succeed_on_arrive.radius
				{
					commands.entity(entity).trigger(OnRunResult::success());
				}
			} else {
				commands.entity(entity).trigger(OnRunResult::failure());
			}
		}
	}
}
