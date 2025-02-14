use beet_flow::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use std::time::Duration;


pub const DEFAULT_ANIMATION_TRANSITION: Duration = Duration::from_millis(250);

/// Play an animation on the agent when this action starts running.
#[action(play_animation_on_run)]
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
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
	/// Lerps into this animation over this duration.
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
	ev: Trigger<OnRun>,
	mut animators: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
	children: Query<&Children>,
	query: Query<&PlayAnimation>,
) {
	let play_animation = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));

	// log::info!("playonrun {}", agents.iter().count());
	// let Ok((mut player, mut transitions)) = agents.get_mut(agent.0) else {
	// 	continue;
	// };
	let target = children
		.iter_descendants_inclusive(ev.origin)
		.find(|entity| animators.contains(*entity))
		.expect(&expect_action::to_have_origin(&ev));
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

/// Play animations for animators that load after the behavior starts
pub(super) fn play_animation_on_load(
	parents: Query<&Parent>,
	mut loaded_animators: Query<
		(Entity, &mut AnimationPlayer, &mut AnimationTransitions),
		Added<AnimationPlayer>,
	>,
	query: Query<(&Running, &PlayAnimation)>,
) {
	for (entity, mut player, mut transitions) in loaded_animators.iter_mut() {
		let Some(play_animation) =
			parents.iter_ancestors_inclusive(entity).find_map(|parent| {
				query.iter().find_map(|(target, play_animation)| {
					if target.origin == parent {
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
