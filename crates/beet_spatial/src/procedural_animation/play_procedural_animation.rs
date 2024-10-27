use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Play a procedural animation with a provided [`SerdeCurve`] for
/// a given [`Duration`]. Add a [`Transform`] to this behavior to
/// offset the target position.
#[derive(Debug, Clone, Component, Reflect, Action)]
#[systems(play_procedural_animation.in_set(TickSet))]
#[reflect(Default, Component, ActionMeta)]
#[require(ContinueRun)]
pub struct PlayProceduralAnimation {
	pub curve: SerdeCurve,
	pub speed: ProceduralAnimationSpeed,
}

impl Default for PlayProceduralAnimation {
	fn default() -> Self {
		Self {
			curve: default(),
			speed: default(),
		}
	}
}

impl PlayProceduralAnimation {
	pub fn with_duration(self, duration: Duration) -> Self {
		Self {
			speed: ProceduralAnimationSpeed::Duration(duration),
			..self
		}
	}
	pub fn with_meter_per_second(self, mps: f32) -> Self {
		Self {
			speed: ProceduralAnimationSpeed::MetersPerSecond(mps),
			..self
		}
	}
}

pub fn play_procedural_animation(
	mut commands: Commands,
	mut transforms: Query<&mut Transform>,
	query: Query<
		(Entity, &PlayProceduralAnimation, &TargetAgent, &RunTimer),
		With<Running>,
	>,
) {
	for (entity, action, target_agent, run_timer) in query.iter() {
		// run_timer.last_started.
		let total_len_meters = action.curve.total_len();
		let t = action.speed.calculate_t(total_len_meters, &run_timer);
		let target_pos = action.curve.sample_clamped(t);

		// if let Ok(transform) = transforms.get(entity) {
		// 	target_pos = transform.transform_point(target_pos);
		// }

		transforms
			.get_mut(target_agent.0)
			.expect(expect_action::TARGET_MISSING)
			.translation = target_pos;

		if t >= 1.0 {
			commands.entity(entity).trigger(OnRunResult::success());
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<PlayProceduralAnimation>::default())
			.insert_time();

		let agent = app.world_mut().spawn(Transform::default()).id();
		app.world_mut().spawn((
			Running,
			PlayProceduralAnimation::default(),
			TargetAgent(agent),
		));

		app.update();

		expect(
			app.world()
				.entity(agent)
				.get::<Transform>()
				.unwrap()
				.translation,
		)
		.to_be(Vec3::new(1., 0., 0.))?;

		app.update_with_millis(500);

		expect(
			app.world()
				.entity(agent)
				.get::<Transform>()
				.unwrap()
				.translation,
		)
		.to_be(Vec3::new(1., 0., 0.))?;

		Ok(())
	}
}
