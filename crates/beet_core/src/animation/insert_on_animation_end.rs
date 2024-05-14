use super::*;
use beet_ecs::prelude::*;
use bevy::animation::RepeatAnimation;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, ActionMeta)]
/// Inserts the given component when an animation is almost finished.
pub struct InsertOnAnimationEnd<T: GenericActionComponent> {
	pub value: T,
	pub animation_clip: Handle<AnimationClip>,
	pub animation_index: AnimationNodeIndex,
	/// The duration of the transition to the next action.
	/// This must be greater than frame delta time or there will be no chance
	/// for the the system to catch the end of the animation.
	pub transition_duration: Duration,
	// is_playing: bool,
}

impl<T: GenericActionComponent> ActionMeta for InsertOnAnimationEnd<T> {
	fn category(&self) -> ActionCategory { ActionCategory::Agent }
}

impl<T: GenericActionComponent> ActionSystems for InsertOnAnimationEnd<T> {
	fn systems() -> SystemConfigs {
		insert_on_animation_end::<T>.in_set(TickSet)
	}
}


impl<T: GenericActionComponent> InsertOnAnimationEnd<T> {
	pub fn new(
		clip: Handle<AnimationClip>,
		index: AnimationNodeIndex,
		value: T,
	) -> Self {
		Self {
			value,
			animation_clip: clip,
			animation_index: index,
			transition_duration: DEFAULT_ANIMATION_TRANSITION,
			// is_playing: false,
		}
	}
	pub fn with_transition_duration(mut self, duration: Duration) -> Self {
		self.transition_duration = duration;
		self
	}
}

pub fn insert_on_animation_end<T: GenericActionComponent>(
	mut commands: Commands,
	animators: Query<&AnimationPlayer>,
	children: Query<&Children>,
	clips: Res<Assets<AnimationClip>>,
	mut query: Query<
		(Entity, &TargetAgent, &mut InsertOnAnimationEnd<T>),
		With<Running>,
	>,
) {
	for (entity, agent, mut action) in query.iter_mut() {
		let Some(target) = ChildrenExt::first(**agent, &children, |entity| {
			animators.contains(entity)
		}) else {
			continue;
		};
		// safe unwrap, just checked
		let player = animators.get(target).unwrap();


		let Some(clip) = clips.get(&action.animation_clip) else {
			continue;
		};
		// let duration = Duration::from_secs_f32(clip.duration());

		let Some(active_animation) = player.animation(action.animation_index)
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
			remaining_time < action.transition_duration.as_secs_f32();


		// let is_playing = player
		// 	.animation(action.animation_index)
		// 	.map(|anim| !anim.is_finished())
		// 	.unwrap_or_default();
		// log::info!("is playing: {is_playing}",);

		// if !nearly_finished && !action.is_playing {
		// action.is_playing = true;
		// } else if nearly_finished && action.is_playing {
		if nearly_finished {
			// action.is_playing = false;
			commands.entity(entity).insert(action.value.clone());
		}
	}
}
