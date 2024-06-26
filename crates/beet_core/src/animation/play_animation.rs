use beet_ecs::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::time::Duration;


pub const DEFAULT_ANIMATION_TRANSITION: Duration = Duration::from_millis(250);

/// Play an animation on the agent when this action starts running.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
pub struct PlayAnimation {
	animation: AnimationNodeIndex,
	/// Trigger once again if the animation is already playing
	pub trigger_if_playing: bool,
	pub repeat: RepeatAnimation,
	pub transition_duration: Duration,
}

impl PlayAnimation {
	pub fn new(animation: AnimationNodeIndex) -> Self {
		Self {
			animation,
			trigger_if_playing: false,
			repeat: RepeatAnimation::default(),
			transition_duration: DEFAULT_ANIMATION_TRANSITION,
		}
	}
	pub fn with_transition_duration(mut self, duration: Duration) -> Self {
		self.transition_duration = duration;
		self
	}
	pub fn repeat(mut self, repeat: RepeatAnimation) -> Self {
		self.repeat = repeat;
		self
	}
	pub fn repeat_forever(mut self) -> Self {
		self.repeat = RepeatAnimation::Forever;
		self
	}
	pub fn trigger_if_playing(mut self) -> Self {
		self.trigger_if_playing = true;
		self
	}
}

/// Play animations for behaviors that run after the agent loads
fn play_animation_on_run(
	mut animators: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
	children: Query<&Children>,
	query: Query<(&TargetAgent, &PlayAnimation), Added<Running>>,
) {
	for (agent, play_animation) in query.iter() {
		// log::info!("playonrun {}", agents.iter().count());
		// let Ok((mut player, mut transitions)) = agents.get_mut(agent.0) else {
		// 	continue;
		// };
		let Some(target) = ChildrenExt::first(**agent, &children, |entity| {
			animators.contains(entity)
		}) else {
			continue;
		};
		// safe unwrap, just checked
		let (mut player, mut transitions) = animators.get_mut(target).unwrap();

		if !player.is_playing_animation(play_animation.animation)
			|| play_animation.trigger_if_playing
		{
			transitions
				.play(
					&mut player,
					play_animation.animation,
					play_animation.transition_duration,
				)
				.set_repeat(play_animation.repeat);
		}
	}
}

/// Play animations for animators that load after the behavior starts
fn play_animation_on_load(
	parents: Query<&Parent>,
	mut loaded_animators: Query<
		(Entity, &mut AnimationPlayer, &mut AnimationTransitions),
		Added<AnimationPlayer>,
	>,
	query: Query<(&TargetAgent, &PlayAnimation), With<Running>>,
) {
	for (entity, mut player, mut transitions) in loaded_animators.iter_mut() {
		let Some(play_animation) =
			ParentExt::find(entity, &parents, |parent| {
				query.iter().find_map(|(target, play_animation)| {
					if **target == parent {
						Some(play_animation)
					} else {
						None
					}
				})
			})
		else {
			continue;
		};
		if !player.is_playing_animation(play_animation.animation)
			|| play_animation.trigger_if_playing
		{
			transitions
				.play(
					&mut player,
					play_animation.animation,
					play_animation.transition_duration,
				)
				.set_repeat(play_animation.repeat);
		}
	}
}

impl ActionMeta for PlayAnimation {
	fn category(&self) -> ActionCategory { ActionCategory::Agent }
}

impl ActionSystems for PlayAnimation {
	fn systems() -> SystemConfigs {
		(play_animation_on_run, play_animation_on_load).in_set(TickSet)
	}
}
