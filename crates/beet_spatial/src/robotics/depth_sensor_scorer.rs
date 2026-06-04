use super::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Sets the [`Score`] based on the [`DepthValue`], usually
/// updated by a sensor.
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

	/// Build a [`ScoreProvider`] that scores by the agent's [`DepthValue`].
	pub fn provider(self) -> ScoreProvider<()> {
		ScoreProvider(Action::<(), Score>::new_async(
			move |cx: ActionContext| {
				let scorer = self.clone();
				async move {
					cx.world()
						.run_system_cached_with(score_depth, (cx.id(), scorer))
						.await?
						.xok()
				}
			},
		))
	}
}

fn score_depth(
	In((action, scorer)): In<(Entity, DepthSensorScorer)>,
	sensors: AgentQuery<&DepthValue>,
) -> Score {
	let Ok(depth) = sensors.get(action) else {
		return scorer.far_score;
	};
	match **depth {
		Some(depth) if depth < scorer.threshold_dist => scorer.close_score,
		_ => scorer.far_score,
	}
}
