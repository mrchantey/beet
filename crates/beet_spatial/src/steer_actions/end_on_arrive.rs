use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Succeeds when the agent arrives at the [`SteerTarget`].
/// Fails if the target is not found.
///
/// A long-running action: while [`Running`] the [`end_on_arrive`] system
/// watches each frame and ends the run with [`Outcome::PASS`] once the
/// agent is within [`EndOnArrive::radius`] of its target. Pair with
/// [`Seek`] on the same entity to drive arrival.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[require(ContinueRun<(), Outcome>)]
#[reflect(Default, Component)]
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

/// Ends any [`Running`] [`EndOnArrive`] whose agent has reached its target.
pub(crate) fn end_on_arrive(
	mut commands: Commands,
	agents: AgentQuery<(&GlobalTransform, &SteerTarget)>,
	transforms: Query<&GlobalTransform>,
	query: Populated<(Entity, &EndOnArrive), With<Running<Outcome>>>,
) -> Result {
	for (action, end_on_arrive) in query.iter() {
		let (transform, target) = agents.get(action).map_err(|_| {
			bevyhow!(
				"EndOnArrive action {action}: its resolved steering agent is missing Transform/SteerTarget"
			)
		})?;
		match target.get_position(&transforms) {
			Ok(target) => {
				if transform.translation().distance_squared(target)
					<= end_on_arrive.radius.squared()
				{
					commands.entity(action).queue(EndRun(Outcome::PASS));
				}
			}
			Err(_) => {
				commands.entity(action).queue(EndRun(Outcome::FAIL));
			}
		}
	}
	Ok(())
}
