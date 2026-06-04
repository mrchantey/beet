use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Provides a [`Score`] based on distance to the [`SteerTarget`].
/// This scorer is binary: if the distance is within the min and max radius
/// the score is `1`, otherwise `0`.
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

impl SteerTargetScoreProvider {
	/// Build a [`ScoreProvider`] that scores by distance to the agent's
	/// [`SteerTarget`].
	pub fn provider(self) -> ScoreProvider<()> {
		ScoreProvider(Action::<(), Score>::new_async(
			move |cx: ActionContext| {
				let Self {
					min_radius,
					max_radius,
				} = self.clone();
				async move {
					cx.world()
						.run_system_cached_with(
							score_steer_target,
							(cx.id(), min_radius, max_radius),
						)
						.await?
						.xok()
				}
			},
		))
	}
}

fn score_steer_target(
	In((action, min_radius, max_radius)): In<(Entity, f32, f32)>,
	transforms: Query<&GlobalTransform>,
	agents: AgentQuery<(&GlobalTransform, &SteerTarget)>,
) -> Score {
	let Ok((transform, target)) = agents.get(action) else {
		return Score::FAIL;
	};
	match target.get_position(&transforms) {
		Ok(target) => {
			let dist = transform.translation().distance_squared(target);
			if dist >= min_radius.squared() && dist <= max_radius.squared() {
				Score::PASS
			} else {
				Score::FAIL
			}
		}
		Err(_) => Score::FAIL,
	}
}
