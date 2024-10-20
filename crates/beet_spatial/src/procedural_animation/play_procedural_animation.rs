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
	pub num_animations: u32,
	pub last_t: f32,
}

impl Default for PlayProceduralAnimation {
	fn default() -> Self {
		Self {
			shape:default(),
			repeat: default(),
			speed: default(),
			num_animations: 0,
			last_t: 0.0,
		}
	}
}

impl PlayProceduralAnimation {
	pub fn get_fraction(&self,time:Res<Time>) -> f32 {
		match self.speed{
			ProceduralAnimationSpeed::MetersPerSecond(mps) => mps * time.delta_seconds(),
			ProceduralAnimationSpeed::FractionPerSecond(fps) => fps,
		}
	}

}

fn play_procedural_animation(
	trigger: Trigger<OnRun>,
	time:Res<Time>,
	mut transforms: Query<&mut Transform>,
	mut query: Query<(&mut PlayProceduralAnimation, &TargetAgent)>,
) {
	let (mut play_procedural_animation, target_agent) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let mut transform = transforms
		.get_mut(target_agent.0)
		.expect(expect_action::TARGET_MISSING);

	// let t = 

}
