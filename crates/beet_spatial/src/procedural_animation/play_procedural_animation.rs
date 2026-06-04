use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use core::time::Duration;

/// Play a procedural animation with a provided [`SerdeCurve`] for
/// a given [`Duration`]. Add a [`Transform`] to this behavior to
/// offset the target position.
///
/// A long-running action: while [`Running`] the [`play_procedural_animation`]
/// system updates the agent's translation along the curve each frame and
/// ends the run with [`Outcome::PASS`] once `t >= 1.0`. Pair with
/// [`SetCurveOnRun`] to regenerate the curve every time the action starts.
#[derive(Debug, Clone, Component, Reflect)]
#[require(ContinueRun<(), Outcome>)]
#[reflect(Default, Component)]
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

/// Steps every [`Running`] [`PlayProceduralAnimation`] along its curve,
/// ending the run with [`Outcome::PASS`] once the parameter reaches `1.0`.
pub(crate) fn play_procedural_animation(
	mut commands: Commands,
	mut agents: AgentQuery<&mut Transform>,
	query: Populated<
		(Entity, &PlayProceduralAnimation, &RunTimer),
		With<Running<Outcome>>,
	>,
) {
	for (action, play_procedural, run_timer) in query.iter() {
		let total_len_meters = play_procedural.curve.total_len();
		let t = play_procedural
			.speed
			.calculate_t(total_len_meters, run_timer);
		let target_pos = play_procedural.curve.sample_clamped(t);

		if let Ok(mut transform) = agents.get_mut(action) {
			transform.translation = target_pos;
		}

		if t >= 1.0 {
			commands.entity(action).queue(EndRun(Outcome::PASS));
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AsyncPlugin,
			ActionPlugin,
			BeetSpatialPlugins,
		));

		let agent = app
			.world_mut()
			.spawn((
				Transform::default(),
				PlayProceduralAnimation::default().with_duration_secs(0.05),
			))
			.id();

		app.world_mut()
			.entity_mut(agent)
			.call::<(), Outcome>(())
			.await
			.unwrap()
			.xpect_eq(Outcome::PASS);
	}
}
