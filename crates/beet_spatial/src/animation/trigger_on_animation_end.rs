use super::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use bevy::animation::RepeatAnimation;
use std::time::Duration;

/// Returns with the provided payload when the specified animation
/// is almost finished.
#[derive(Debug, Clone, Component)]
#[require(ContinueRun)]
pub struct TriggerOnAnimationEnd<P> {
	/// The result payload to return when the animation ends.
	pub payload: P,
	/// The animation clip to check for end.
	pub handle: Handle<AnimationClip>,
	/// The index of the animation node to check for end.
	pub animation_index: AnimationNodeIndex,
	/// The duration of the transition to the next action.
	/// This should be greater than frame delta time or there will be no chance
	/// for the the system to catch the end of the animation.
	pub transition_duration: Duration,
}


impl<P> TriggerOnAnimationEnd<P> {
	/// Create a new [`TriggerOnAnimationEnd`] action.
	pub fn new(
		handle: Handle<AnimationClip>,
		index: AnimationNodeIndex,
		payload: P,
	) -> Self {
		Self {
			payload,
			handle,
			animation_index: index,
			transition_duration: DEFAULT_ANIMATION_TRANSITION,
		}
	}
	/// The duration before the end of the animation to trigger the event.
	/// This is commonly the same as the transition duration of the animation.
	pub fn with_transition_duration(mut self, duration: Duration) -> Self {
		self.transition_duration = duration;
		self
	}
}

pub(crate) fn trigger_on_animation_end<P: EventPayload + Clone>(
	mut commands: Commands,
	animators: Query<&AnimationPlayer>,
	children: Query<&Children>,
	clips: When<Res<Assets<AnimationClip>>>,
	mut query: Populated<(Entity, &TriggerOnAnimationEnd<P>), With<Running>>,
) {
	for (action, on_end) in query.iter_mut() {
		println!("1");
		let Some(target) = children
			.iter_descendants_inclusive(action)
			.find(|entity| animators.contains(*entity))
		else {
			continue;
		};
		println!("2");
		// safe unwrap, just checked
		let player = animators.get(target).unwrap();

		let Some(clip) = clips.get(&on_end.handle) else {
			continue;
		};

		let Some(active_animation) = player.animation(on_end.animation_index)
		else {
			continue;
		};


		println!("3");
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

		let duration = on_end.transition_duration.as_secs_f32();

		let nearly_finished = remaining_time < duration;

		println!("remaining time: {:.1}/{:.1}", remaining_time, duration);
		if nearly_finished {
			println!("Animation ended!");
			commands
				.entity(action)
				.trigger_payload(on_end.payload.clone());
		}
	}
}
