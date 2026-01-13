use super::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

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
	pub far_score: Score,
	/// The score to set when the depth is less than the threshold.
	pub close_score: Score,
}

impl Default for DepthSensorScorer {
	fn default() -> Self {
		Self {
			threshold_dist: 0.5,
			far_score: Score::FAIL,
			close_score: Score::PASS,
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
	ev: On<GetScore>,
	mut commands: Commands,
	query: Query<&DepthSensorScorer>,
	sensors: AgentQuery<&DepthValue, Changed<DepthValue>>,
) -> Result {
	let target = ev.target();
	let scorer = query.get(target)?;
	let depth = sensors.get(target)?;
	let next_score = if let Some(depth) = **depth {
		if depth < scorer.threshold_dist {
			scorer.close_score
		} else {
			scorer.far_score
		}
	} else {
		scorer.far_score
	};
	commands.entity(target).trigger_target(next_score);
	Ok(())
}
