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
#[require(ContinueRun, RunTimer)]
pub struct PlayProceduralAnimation {
	pub curve: SerdeCurve,
	/// t per second, 1.0 will complete the curve in 1 second
	pub duration: Duration,
}

impl Default for PlayProceduralAnimation {
	fn default() -> Self {
		Self {
			curve: default(),
			duration: Duration::from_secs(1),
		}
	}
}

impl PlayProceduralAnimation {
	pub fn with_duration(self, duration: Duration) -> Self {
		Self { duration, ..self }
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
		let t = run_timer.last_started.elapsed().as_secs_f32()
			/ action.duration.as_secs_f32();
		let mut target_pos = action.curve.sample_clamped(t);

		if let Ok(transform) = transforms.get(entity) {
			target_pos = transform.transform_point(target_pos);
		}

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
