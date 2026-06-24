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
#[derive(Debug, Clone, Component, Reflect)]
#[require(ContinueRun<(), P>)]
#[reflect(Component, Default)]
pub struct TriggerOnAnimationEnd<P>
where
	P: 'static
		+ Send
		+ Sync
		+ Clone
		+ Default
		+ Reflect
		+ FromReflect
		+ TypePath,
{
	/// The result payload to return when the animation ends.
	pub payload: P,
	/// Path to the animation clip, resolved to a graph node index against the
	/// agent's [`AnimationGraphClips`] each frame.
	pub clip: SmolStr,
	/// The duration before the animation ends to trigger the action.
	/// This should usually match the [`AnimationTransitions`] duration for
	/// smooth crossfade.
	pub transition_duration: Duration,
}

impl<P> Default for TriggerOnAnimationEnd<P>
where
	P: 'static
		+ Send
		+ Sync
		+ Clone
		+ Default
		+ Reflect
		+ FromReflect
		+ TypePath,
{
	fn default() -> Self {
		Self {
			payload: P::default(),
			clip: SmolStr::default(),
			transition_duration: DEFAULT_ANIMATION_TRANSITION,
		}
	}
}

/// Ends any [`Running`] [`TriggerOnAnimationEnd`] whose active animation is
/// within `transition_duration` of completing.
pub(crate) fn trigger_on_animation_end<P>(
	mut commands: Commands,
	clips: When<Res<Assets<AnimationClip>>>,
	asset_server: When<Res<AssetServer>>,
	graph_clips: Query<&AnimationGraphClips>,
	query: Populated<(Entity, &TriggerOnAnimationEnd<P>), With<Running<P>>>,
	agents: AgentQuery<&AnimationPlayer>,
) -> Result
where
	P: 'static
		+ Send
		+ Sync
		+ Clone
		+ Default
		+ Reflect
		+ FromReflect
		+ TypePath,
{
	for (action, on_end) in query.iter() {
		// resolve the clip path to a node index and asset handle on the agent root
		let agent = agents.entity(action);
		let animation_index = graph_clips
			.get(agent)
			.map_err(|_| {
				bevyhow!(
					"TriggerOnAnimationEnd on {action} has no AnimationGraphClips on agent root {agent}"
				)
			})?
			.index(&on_end.clip)
			.ok_or_else(|| {
				bevyhow!("clip `{}` not in agent's AnimationGraph", on_end.clip)
			})?;
		let handle = asset_server.load::<AnimationClip>(on_end.clip.to_string());
		let player = agents.get_descendent(action)?;
		// assets are gated to be loaded before LoadTemplate, so a missing clip
		// asset is a real error, not a wait.
		let clip = clips.get(&handle).ok_or_else(|| {
			bevyhow!("clip `{}` not loaded", on_end.clip)
		})?;

		let Some(active_animation) = player.animation(animation_index) else {
			// not playing yet: legitimately transient
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
