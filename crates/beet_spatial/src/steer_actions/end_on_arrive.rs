use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


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
	agents: AgentQuery<(&GlobalTransform, &SteerTarget)>,
	transforms: Query<&GlobalTransform>,
	mut query: Query<(Entity, &EndOnArrive), With<Running>>,
) -> Result {
	for (action, end_on_arrive) in query.iter_mut() {
		let (transform, target) = agents.get(action)?;
		if let Ok(target) = target.get_position(&transforms) {
			if transform.translation().distance_squared(target)
				<= end_on_arrive.radius.powi(2)
			{
				commands.entity(action).trigger_target(Outcome::Pass);
			}
		} else {
			commands.entity(action).trigger_target(Outcome::Fail);
		}
	}
	Ok(())
}
