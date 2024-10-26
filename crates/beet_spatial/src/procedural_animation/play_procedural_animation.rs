use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;


#[derive(Debug, Clone, PartialEq, Component, Reflect, Action)]
#[observers(play_procedural_animation)]
#[reflect(Default, Component, ActionMeta)]
pub struct PlayProceduralAnimation {
	pub curve: SerdeCurve,
	/// t per second, 1.0 will complete the curve in 1 second
	pub speed: f32,
	pub repeat: RepeatAnimation,
	pub completions: u32,
	pub elapsed_t: f32,
}

impl Default for PlayProceduralAnimation {
	fn default() -> Self {
		Self {
			curve: default(),
			repeat: default(),
			speed: 1.,
			completions: 0,
			elapsed_t: 0.,
		}
	}
}

impl PlayProceduralAnimation {
	pub fn is_finished(&self) -> bool {
		match self.repeat {
			RepeatAnimation::Forever => false,
			RepeatAnimation::Never => self.completions >= 1,
			RepeatAnimation::Count(n) => self.completions >= n,
		}
	}
}

pub fn play_procedural_animation(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	mut query: Query<(&mut PlayProceduralAnimation, &TargetAgent)>,
) {
	println!("🚀🚀🚀");
	let (mut action, target_agent) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let mut transform = transforms
		.get_mut(target_agent.0)
		.expect(expect_action::TARGET_MISSING);

	action.elapsed_t += action.speed * time.delta_secs();
	transform.translation = action.curve.sample_clamped(action.elapsed_t);

	if action.elapsed_t >= 1.0 {
		action.completions += 1;
		if action.is_finished() {
			commands
				.entity(trigger.entity())
				.insert(OnRunResult::success());
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
		let behavior = app
			.world_mut()
			.spawn((PlayProceduralAnimation::default(), TargetAgent(agent)))
			.id();

		app.world_mut().entity_mut(behavior).flush_trigger(OnRun);

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
