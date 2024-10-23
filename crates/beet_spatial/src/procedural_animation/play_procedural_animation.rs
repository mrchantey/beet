use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;


#[derive(Debug, Clone, PartialEq, Component, Reflect, Action)]
#[observers(play_procedural_animation)]
#[reflect(Default, Component, ActionMeta)]
pub struct PlayProceduralAnimation {
	pub shape: ProceduralAnimationShape,
	pub speed: ProceduralAnimationSpeed,
	pub repeat: RepeatAnimation,
	pub completions: u32,
	pub elapsed_t: f32,
}

impl Default for PlayProceduralAnimation {
	fn default() -> Self {
		Self {
			shape: default(),
			repeat: default(),
			speed: default(),
			completions: 0,
			elapsed_t: 0.0,
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
	pub fn get_fraction(&self, time: Res<Time>) -> f32 {
		match self.speed {
			ProceduralAnimationSpeed::FractionPerSecond(d_t) => {
				d_t * time.delta_secs()
			}
			ProceduralAnimationSpeed::MetersPerSecond(d_m) => {
				(d_m * time.delta_secs()) / self.shape.total_length()
			}
		}
	}
}

fn play_procedural_animation(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	mut query: Query<(&mut PlayProceduralAnimation, &TargetAgent)>,
) {
	let (mut action, target_agent) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let mut transform = transforms
		.get_mut(target_agent.0)
		.expect(expect_action::TARGET_MISSING);

	let t = action.elapsed_t + action.get_fraction(time);
	action.elapsed_t = t;

	if t >= 1.0 {
		action.completions += 1;
		if action.is_finished() {
			commands
				.entity(trigger.entity())
				.insert(OnRunResult::success());
		}
	}

	transform.translation = action.shape.fraction_to_pos(t);
}


#[cfg(test)]
mod test {
	use anyhow::Result;
	use sweet::*;
	
	#[test]
	fn works() -> Result<()> {
		expect(true).to_be_false()?;
		
		Ok(())
	}
	
}