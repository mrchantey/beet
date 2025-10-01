use super::*;
use beet_flow::prelude::*;
use beet_core::prelude::*;

/// Sets the [`Score`] based on the [`DepthValue`], usually
/// updated by a sensor.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
#[action(depth_sensor_scorer)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct DepthSensorScorer {
	/// The distance at which the sensor will toggle from
	/// `far_score` to `close_score`.
	pub threshold_dist: f32,
	/// The score to set when the depth is more than the threshold.
	pub far_score: ScoreValue,
	/// The score to set when the depth is less than the threshold.
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
	/// Create a new depth sensor scorer with the given threshold distance.
	pub fn new(threshold_dist: f32) -> Self {
		Self {
			threshold_dist,
			..Default::default()
		}
	}
}

fn depth_sensor_scorer(
	ev: On<OnRun<RequestScore>>,
	mut commands: Commands,
	sensors: Query<&DepthValue, Changed<DepthValue>>,
	query: Query<&DepthSensorScorer>,
) {
	let scorer = query
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
	ev.trigger_result(&mut commands, next_score);
}
