use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use std::time::Duration;



/// Convenience bundle for common collection of actions and components
/// required to play an animation on run and end the run on animation end.
#[derive(Bundle)]
pub struct AnimationActionBundle {
	clip: AssetPlaceholder<AnimationClip>,
	play_animation: PlayAnimation,
	on_end: TriggerOnAnimationEnd<OnRunResult>,
}


impl AnimationActionBundle {
	pub fn new(
		graph: &mut AnimationGraphPlaceholder,
		clip: impl Into<String>,
	) -> Self {
		let clip = AssetPlaceholder::<AnimationClip>::new(clip);
		let index = graph.add_clip(clip.clone(), 1.0, graph.root);
		let transition_duration = Duration::from_millis(500);

		Self {
			clip,
			play_animation: PlayAnimation::new(index)
				.with_transition_duration(transition_duration),
			on_end: TriggerOnAnimationEnd::new(
				index,
				// should this specify the action?
				// OnResultAction::local(RunResult::Success),
			)
			.with_transition_duration(transition_duration),
		}
	}

	pub fn repeat(mut self, repeat: RepeatAnimation) -> Self {
		self.play_animation = self.play_animation.repeat(repeat);
		self
	}


	pub fn with_transition_duration(mut self, duration: Duration) -> Self {
		self.play_animation =
			self.play_animation.with_transition_duration(duration);
		self.on_end = self.on_end.with_transition_duration(duration);
		self
	}
}
