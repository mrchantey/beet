use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
/// Adjusts the [`Score`] based on distance to the [`SteerTarget`]
pub struct ScoreSteerTarget {
	pub radius: f32,
}

impl Default for ScoreSteerTarget {
	fn default() -> Self { Self { radius: 0.5 } }
}

impl ScoreSteerTarget {
	pub fn new(radius: f32) -> Self { Self { radius } }
}

fn score_steer_target(
	transforms: Query<&Transform>,
	agents: Query<(&Transform, &SteerTarget)>,
	mut query: Query<(&TargetAgent, &ScoreSteerTarget, &mut Score)>,
) {
	for (agent, scorer, mut score) in query.iter_mut() {
		if let Ok((transform, target)) = agents.get(**agent) {
			if let Ok(target) = target.position(&transforms) {
				if Vec3::distance(transform.translation, target)
					<= scorer.radius
				{
					*score = Score::Pass;
					continue;
				}
			}
		}
		*score = Score::Fail;
	}
}
// Or<(
// 	Changed<Transform>,
// 	Changed<SteerTarget>,
// 	Changed<ScoreSteerTarget>,
// )>,

impl ActionMeta for ScoreSteerTarget {
	fn category(&self) -> ActionCategory { ActionCategory::Internal }
}

impl ActionSystems for ScoreSteerTarget {
	fn systems() -> SystemConfigs { score_steer_target.in_set(PreTickSet) }
}
