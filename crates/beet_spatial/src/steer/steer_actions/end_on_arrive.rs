use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;


/// Succeeds when the agent arrives at the [`SteerTarget`].
/// Fails if the target is not found.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(end_on_arrive.in_set(TickSet))]
#[require(ContinueRun)]
pub struct EndOnArrive {
	pub radius: f32,
}

impl Default for EndOnArrive {
	fn default() -> Self { Self { radius: 0.5 } }
}

impl EndOnArrive {
	pub fn new(radius: f32) -> Self { Self { radius } }
}

pub fn end_on_arrive(
	mut commands: Commands,
	agents: Query<(&GlobalTransform, &SteerTarget)>,
	transforms: Query<&GlobalTransform>,
	mut query: Query<(Entity, &TargetEntity, &EndOnArrive), With<Running>>,
) {
	for (entity, agent, action) in query.iter_mut() {
		if let Ok((transform, target)) = agents.get(**agent) {
			if let Ok(target) = target.get_position(&transforms) {
				if transform.translation().distance_squared(target)
					<= action.radius.powi(2)
				{
					commands.entity(entity).trigger(OnRunResult::success());
				}
			} else {
				commands.entity(entity).trigger(OnRunResult::failure());
			}
		}
	}
}
