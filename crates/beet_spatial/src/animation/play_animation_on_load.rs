use crate::prelude::*;
use beet_core::prelude::*;
use bevy::animation::RepeatAnimation;

/// Plays an animation on a model as soon as its [`AnimationPlayer`] is ready, looping
/// it by default, then removes itself.
///
/// The declarative, behaviour-tree-free way to "just play this animation forever":
/// where [`PlayAnimation`] is an action a `<Sequence>` runs, this is a plain
/// component a scene spreads onto a model (eg `<Foxie {PlayAnimationOnLoad{clip:..}}/>`)
/// to keep it idling with no `RunOnLoad`/`Sequence` at all. Resolves the clip against
/// the entity's own [`AnimationGraphClips`] and plays it on the [`AnimationPlayer`]
/// the glb spawns as a descendant; it is a one-shot, so the model can later be driven
/// by an action without contention.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct PlayAnimationOnLoad {
	/// Path to the animation clip, resolved against this entity's
	/// [`AnimationGraphClips`].
	pub clip: SmolStr,
	/// How many times to repeat (defaults to [`RepeatAnimation::Forever`]).
	pub repeat: RepeatAnimation,
}

impl Default for PlayAnimationOnLoad {
	fn default() -> Self {
		Self {
			clip: SmolStr::default(),
			repeat: RepeatAnimation::Forever,
		}
	}
}

impl PlayAnimationOnLoad {
	/// Loop the clip at `clip` forever once the player is ready.
	pub fn new(clip: impl Into<SmolStr>) -> Self {
		Self {
			clip: clip.into(),
			..Default::default()
		}
	}
}

/// Plays each [`PlayAnimationOnLoad`] clip once its descendant [`AnimationPlayer`]
/// exists (the glb scene loads async), then removes the component so it runs once.
pub(crate) fn play_animation_on_load(
	mut commands: Commands,
	query: Query<(Entity, &PlayAnimationOnLoad)>,
	graph_clips: Query<&AnimationGraphClips>,
	children: Query<&Children>,
	mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
	for (entity, on_load) in query.iter() {
		let Ok(clips) = graph_clips.get(entity) else {
			continue;
		};
		let Some(index) = clips.index(on_load.clip.as_str()) else {
			continue;
		};
		for descendant in children.iter_descendants_inclusive(entity) {
			if let Ok((mut player, mut transitions)) = players.get_mut(descendant)
			{
				transitions
					.play(&mut player, index, core::time::Duration::ZERO)
					.set_repeat(on_load.repeat);
				commands.entity(entity).remove::<PlayAnimationOnLoad>();
				break;
			}
		}
	}
}
