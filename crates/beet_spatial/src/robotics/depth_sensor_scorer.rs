use super::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Sets the [`Score`] based on the [`DepthValue`].
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(depth_sensor_scorer)]
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
			far_score: score::FAIL,
			close_score: score::PASS,
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
	trigger: Trigger<RequestScore>,
	mut commands: Commands,
	sensors: Query<&DepthValue, Changed<DepthValue>>,
	query: Query<(&TargetEntity, &DepthSensorScorer, &Parent)>,
) {
	let (target, scorer, parent) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);
	if let Ok(depth) = sensors.get(**target) {
		let next_score = if let Some(depth) = **depth {
			if depth < scorer.threshold_dist {
				scorer.close_score
			} else {
				scorer.far_score
			}
		} else {
			scorer.far_score
		};
		commands
			.entity(parent.get())
			.trigger(OnChildScore::new(trigger.entity(), next_score));
	}
}
