use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_transform::components::Transform;
use serde::Deserialize;
use serde::Serialize;

#[action(system=score_steer_target)]
pub struct ScoreSteerTarget {
	pub radius: f32,
	#[shared]
	pub score: Score,
}

impl Default for ScoreSteerTarget {
	fn default() -> Self {
		Self {
			score: Score::Fail,
			radius: 0.5,
		}
	}
}

fn score_steer_target(
	transforms: Query<&Transform>,
	mut query: Query<
		(&Transform, Option<&SteerTarget>, &mut ScoreSteerTarget),
		With<Running>,
	>,
) {
	for (transform, target, mut scorer) in query.iter_mut() {
		if let Some(target) = target {
			if let Ok(target) = target.position(&transforms) {
				if Vec3::distance(transform.translation, target)
					<= scorer.radius
				{
					scorer.score = Score::Pass;
					continue;
				}
			}
		}
		scorer.score = Score::Fail;
	}
}
// Or<(
// 	Changed<Transform>,
// 	Changed<SteerTarget>,
// 	Changed<ScoreSteerTarget>,
// )>,
