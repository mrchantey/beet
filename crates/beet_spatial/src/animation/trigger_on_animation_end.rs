use super::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::animation::RepeatAnimation;
use core::time::Duration;

/// Returns with the provided payload when the specified animation
/// is almost finished.
///
/// A long-running action: while [`Running`] the [`trigger_on_animation_end`]
/// system watches the agent's [`AnimationPlayer`] each frame and ends the
/// run with `payload` once the active animation is within
/// [`transition_duration`] of finishing.
///
/// [`transition_duration`]: TriggerOnAnimationEnd::transition_duration
#[derive(Debug, Clone, Component)]
#[require(ContinueRun<(), P>)]
pub struct TriggerOnAnimationEnd<P>
where
	P: 'static + Send + Sync + Clone,
{
	/// The result payload to return when the animation ends.
	pub payload: P,
	/// The animation clip to check for end.
	pub handle: Handle<AnimationClip>,
	/// The index of the animation to check for end.
	pub animation_index: AnimationNodeIndex,
	/// The duration before the animation ends to trigger the action.
	/// This should usually match the [`AnimationTransitions`] duration for
	/// smooth crossfade.
	pub transition_duration: Duration,
}


impl<P> TriggerOnAnimationEnd<P>
where
	P: 'static + Send + Sync + Clone,
{
	/// Create a new [`TriggerOnAnimationEnd`] action.
	pub fn new(
		handle: Handle<AnimationClip>,
		animation_index: AnimationNodeIndex,
		payload: P,
	) -> Self {
		Self {
			handle,
			animation_index,
			payload,
			transition_duration: DEFAULT_ANIMATION_TRANSITION,
		}
	}

	/// Set the [`Self::transition_duration`]
	pub fn with_transition_duration(mut self, duration: Duration) -> Self {
		self.transition_duration = duration;
		self
	}
}

/// Ends any [`Running`] [`TriggerOnAnimationEnd`] whose active animation is
/// within `transition_duration` of completing.
pub(crate) fn trigger_on_animation_end<P>(
	mut commands: Commands,
	clips: When<Res<Assets<AnimationClip>>>,
	query: Populated<(Entity, &TriggerOnAnimationEnd<P>), With<Running<P>>>,
	agents: AgentQuery<&AnimationPlayer>,
) -> Result
where
	P: 'static + Send + Sync + Clone,
{
	for (action, on_end) in query.iter() {
		let player = agents.get_descendent(action)?;
		let clip = clips
			.get(&on_end.handle)
			.ok_or_else(|| bevyhow!("clip not found"))?;

		let Some(active_animation) = player.animation(on_end.animation_index)
		else {
			// animation not playing yet
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

		if remaining_time < on_end.transition_duration.as_secs_f32() {
			commands
				.entity(action)
				.queue(EndRun(on_end.payload.clone()));
		}
	}
	Ok(())
}
