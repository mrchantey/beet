use beet_ecs::prelude::*;
use bevy::prelude::*;

pub const DEFAULT_ULTRASOUND_MAX_DEPTH: f32 = 2.0;

#[derive(
	Debug, Default, Clone, Deref, DerefMut, Component, PartialEq, Reflect,
)]
pub struct DepthValue(pub Option<f32>);

impl DepthValue {
	pub fn new(value: f32) -> Self { Self(Some(value)) }
}

#[derive_action(Default)]
/// Sets the [`Score`] based on the [`DepthValue`].
pub struct DepthSensorScorer {
	#[inspector(step = 0.1)]
	pub threshold_dist: f32,
	pub far_score: Score,
	pub close_score: Score,
}

impl Default for DepthSensorScorer {
	fn default() -> Self {
		Self {
			threshold_dist: 0.5,
			far_score: Score::Fail,
			close_score: Score::Pass,
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

pub fn depth_sensor_scorer(
	sensors: Query<&DepthValue, Changed<DepthValue>>,
	mut scorers: Query<(&TargetAgent, &DepthSensorScorer, &mut Score)>,
) {
	for (target, scorer, mut score) in scorers.iter_mut() {
		if let Ok(depth) = sensors.get(**target) {
			let next_score = if let Some(depth) = **depth
				&& depth < scorer.threshold_dist
			{
				scorer.close_score
			} else {
				scorer.far_score
			};
			score.set_if_neq(next_score);
		}
	}
}
