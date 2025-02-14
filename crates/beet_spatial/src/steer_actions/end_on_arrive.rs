use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;


/// Succeeds when the agent arrives at the [`SteerTarget`].
/// Fails if the target is not found.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct EndOnArrive {
	/// The radius at which the agent should arrive, defaults to `0.5`
	pub radius: f32,
}

impl Default for EndOnArrive {
	fn default() -> Self { Self { radius: 0.5 } }
}

impl EndOnArrive {
	/// Create a new [`EndOnArrive`] action with the given radius
	pub fn new(radius: f32) -> Self { Self { radius } }
}

pub(crate) fn end_on_arrive(
	mut commands: Commands,
	agents: Query<(&GlobalTransform, &SteerTarget)>,
	transforms: Query<&GlobalTransform>,
	mut query: Query<(Entity, &Running, &EndOnArrive), With<Running>>,
) {
	for (action, running, end_on_arrive) in query.iter_mut() {
		let (transform, target) = agents
			.get(running.origin)
			.expect(&expect_action::to_have_origin(&running));
		if let Ok(target) = target.get_position(&transforms) {
			if transform.translation().distance_squared(target)
				<= end_on_arrive.radius.powi(2)
			{
				running.trigger_result(
					&mut commands,
					action,
					RunResult::Success,
				);
			}
		} else {
			running.trigger_result(&mut commands, action, RunResult::Failure);
		}
	}
}
