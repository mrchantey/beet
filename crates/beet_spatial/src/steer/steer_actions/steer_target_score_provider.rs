use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

#[action(provide_score)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
/// Provides a [`ScoreValue`] based on distance to the [`SteerTarget`]
pub struct SteerTargetScoreProvider {
	/// fail if already at location
	pub min_radius: f32,
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
	ev: Trigger<OnRun<RequestScore>>,
	commands: Commands,
	transforms: Query<&GlobalTransform>,
	agents: Query<(&GlobalTransform, &SteerTarget)>,
	query: Query<&SteerTargetScoreProvider>,
) {
	let action = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	let (transform, target) = agents
		.get(ev.origin)
		.expect(&expect_action::to_have_origin(&ev));
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
	ev.trigger_result(commands, ScoreValue::new(score));
}
