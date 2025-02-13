use super::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Sets the [`Score`] based on the [`DepthValue`].
#[action(depth_sensor_scorer)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct DepthSensorScorer {
	// #[inspector(step = 0.1)]
	pub threshold_dist: f32,
	pub far_score: ScoreValue,
	pub close_score: ScoreValue,
}

impl Default for DepthSensorScorer {
	fn default() -> Self {
		Self {
			threshold_dist: 0.5,
			far_score: ScoreValue::FAIL,
			close_score: ScoreValue::PASS,
		}
	}
}

impl DepthSensorScorer {
	pub fn new(threshold_dist: f32) -> Self {
		Self {
			threshold_dist,
			..Default::default()
		}
	}
}

fn depth_sensor_scorer(
	ev: Trigger<OnRun<RequestScore>>,
	mut commands: Commands,
	sensors: Query<&DepthValue, Changed<DepthValue>>,
	query: Query<(&DepthSensorScorer, &Parent)>,
) {
	let (scorer, parent) = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	let depth = sensors
		.get(ev.origin)
		.expect(&expect_action::to_have_origin(&ev));
	let next_score = if let Some(depth) = **depth {
		if depth < scorer.threshold_dist {
			scorer.close_score
		} else {
			scorer.far_score
		}
	} else {
		scorer.far_score
	};
	ev.trigger_result(commands, next_score);
}
