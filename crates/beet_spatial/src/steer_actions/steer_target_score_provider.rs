use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Provides a [`ScoreValue`] based on distance to the [`SteerTarget`],
/// This scorer is binary, if the distance is within the min and max radius, the score is 1,
/// otherwise it is 0.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
#[action(provide_score)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct SteerTargetScoreProvider {
	/// if the distance is less than this, the score is 0.
	pub min_radius: f32,
	/// If the distance is greater than this, the score is 0.
	pub max_radius: f32,
}

impl Default for SteerTargetScoreProvider {
	fn default() -> Self {
		Self {
			min_radius: 1.,
			max_radius: 10.,
		}
	}
}

fn provide_score(
	ev: On<Run<RequestScore>>,
	mut commands: Commands,
	transforms: Query<&GlobalTransform>,
	agents: AgentQuery<(&GlobalTransform, &SteerTarget)>,
	query: Query<&SteerTargetScoreProvider>,
) -> Result {
	let action = query.get(ev.event_target())?;
	let (transform, target) = agents.get(ev.event_target())?;
	let score = if let Ok(target) = target.get_position(&transforms) {
		let dist = transform.translation().distance_squared(target);
		if dist >= action.min_radius.powi(2)
			&& dist <= action.max_radius.powi(2)
		{
			1.
		} else {
			0.
		}
	} else {
		0.
	};
	ev.trigger_result(&mut commands, ScoreValue::new(score));
	Ok(())
}
