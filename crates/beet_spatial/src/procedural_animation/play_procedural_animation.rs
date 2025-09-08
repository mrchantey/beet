use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Play a procedural animation with a provided [`SerdeCurve`] for
/// a given [`Duration`]. Add a [`Transform`] to this behavior to
/// offset the target position.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct PlayProceduralAnimation {
	/// The type of curve to animate along.
	pub curve: SerdeCurve,
	/// The speed of the animation, either as a [`Duration`] or in meters per second.
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
	/// Set the speed of the animation to a given duration in seconds.
	pub fn with_duration_secs(self, secs: f32) -> Self {
		Self {
			speed: ProceduralAnimationSpeed::Duration(Duration::from_secs_f32(
				secs,
			)),
			..self
		}
	}
	/// Set the speed of the animation to a given duration in meters per second.
	pub fn with_meter_per_second(self, mps: f32) -> Self {
		Self {
			speed: ProceduralAnimationSpeed::MetersPerSecond(mps),
			..self
		}
	}
	/// Set the curve to animate along.
	pub fn with_curve(self, curve: SerdeCurve) -> Self {
		Self { curve, ..self }
	}
}

pub(crate) fn play_procedural_animation(
	mut commands: Commands,
	mut transforms: Query<&mut Transform>,
	query: Query<(Entity, &PlayProceduralAnimation, &Running, &RunTimer)>,
) {
	for (action, play_procedural, running, run_timer) in query.iter() {
		// run_timer.last_started.
		let total_len_meters = play_procedural.curve.total_len();
		let t = play_procedural
			.speed
			.calculate_t(total_len_meters, &run_timer);
		let target_pos = play_procedural.curve.sample_clamped(t);

		// if let Ok(transform) = transforms.get(entity) {
		// 	target_pos = transform.transform_point(target_pos);
		// }

		transforms
			.get_mut(running.origin)
			.expect(&expect_action::to_have_origin(&running))
			.translation = target_pos;

		if t >= 1.0 {
			running.trigger_result(&mut commands, action, RunResult::Success);
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins((
			BeetFlowPlugin::default(),
			BeetSpatialPlugins::default(),
		))
		.insert_time();

		let agent = app
			.world_mut()
			.spawn((
				Transform::default(),
				Running::default(),
				PlayProceduralAnimation::default(),
			))
			.id();

		app.update();

		app.world()
			.entity(agent)
			.get::<Transform>()
			.unwrap()
			.translation
			.xpect()
			.to_be(Vec3::new(1., 0., 0.));

		app.update_with_millis(500);

		app.world()
			.entity(agent)
			.get::<Transform>()
			.unwrap()
			.translation
			.xpect()
			.to_be_close_to(Vec3::new(-1., 0., 0.));
	}
}
