use super::*;
use beet_flow::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[require(ContinueRun)]
/// Inserts the given component when an animation is almost finished.
/// Requires a [`Handle<AnimationClip>`] component.
pub struct TriggerOnAnimationEnd<T> {
	pub value: T,
	pub target: ActionTarget,
	pub animation_index: AnimationNodeIndex,
	/// The duration of the transition to the next action.
	/// This should be greater than frame delta time or there will be no chance
	/// for the the system to catch the end of the animation.
	pub transition_duration: Duration,
}


impl<T: Event> TriggerOnAnimationEnd<T> {
	pub fn new(index: AnimationNodeIndex, value: T) -> Self {
		Self {
			value,
			target: ActionTarget::default(),
			animation_index: index,
			transition_duration: DEFAULT_ANIMATION_TRANSITION,
		}
	}
	pub fn with_target(mut self, target: ActionTarget) -> Self {
		self.target = target;
		self
	}
	/// The duration before the end of the animation to trigger the event.
	/// This is commonly the same as the transition duration of the animation.
	pub fn with_transition_duration(mut self, duration: Duration) -> Self {
		self.transition_duration = duration;
		self
	}
}

pub fn trigger_on_animation_end<T: Event>(
	mut commands: Commands,
	animators: Query<&AnimationPlayer>,
	children: Query<&Children>,
	clips: Res<Assets<AnimationClip>>,
	mut query: Query<(
		Entity,
		&Running,
		&TriggerOnAnimationEnd<T>,
		&Handle<AnimationClip>,
	)>,
) {
	for (action, running, trigger_on_end, handle) in query.iter_mut() {
		let Some(target) = children
			.iter_descendants_inclusive(running.origin)
			.find(|entity| animators.contains(*entity))
		else {
			continue;
		};
		// safe unwrap, just checked
		let player = animators.get(target).unwrap();

		let Some(clip) = clips.get(&**handle) else {
			continue;
		};

		let Some(active_animation) =
			player.animation(trigger_on_end.animation_index)
		else {
			continue;
		};


		let remaining_time = match active_animation.repeat_mode() {
			RepeatAnimation::Never => {
				clip.duration() - active_animation.seek_time()
			}
			RepeatAnimation::Count(count) => {
				let total = clip.duration() * count as f32;
				let current = clip.duration()
					* active_animation.completions() as f32
					+ active_animation.seek_time();
				total - current
			}
			RepeatAnimation::Forever => f32::INFINITY,
		};

		let nearly_finished =
			remaining_time < trigger_on_end.transition_duration.as_secs_f32();

		if nearly_finished {
			trigger_on_end.target.trigger(
				&mut commands,
				action,
				trigger_on_end.value.clone(),
			);
			// commands.entity(entity).trigger();
		}
	}
}
