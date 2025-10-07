use beet_core::prelude::*;
use beet_flow::prelude::*;
use bevy::animation::RepeatAnimation;
use std::time::Duration;


pub(super) const DEFAULT_ANIMATION_TRANSITION: Duration =
	Duration::from_millis(250);

/// Play an animation on the agent when this action starts running.
#[action(play_animation_on_run)]
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct PlayAnimation {
	animation: AnimationNodeIndex,
	/// Trigger once again if the animation is already playing
	pub trigger_if_playing: bool,
	/// Amount of times to repeat the animation.
	pub repeat: RepeatAnimation,
	/// The crossfade duration, ie the duration before previous animation
	/// end to start the next one.
	pub transition_duration: Duration,
}

impl PlayAnimation {
	/// Create a new [`PlayAnimation`] action.
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
	/// Repeat the animation a set number of times.
	pub fn repeat(mut self, repeat: RepeatAnimation) -> Self {
		self.repeat = repeat;
		self
	}
	/// Repeat the animation forever.
	pub fn repeat_forever(mut self) -> Self {
		self.repeat = RepeatAnimation::Forever;
		self
	}
	/// Trigger the animation even if it is already playing.
	pub fn trigger_if_playing(mut self) -> Self {
		self.trigger_if_playing = true;
		self
	}
}

/// Play animations for behaviors that run after the agent loads
fn play_animation_on_run(
	ev: On<GetOutcome>,
	query: Query<&PlayAnimation>,
	mut agents: AgentQuery<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) -> Result {
	let play_animation = query.get(ev.event_target())?;
	let (mut player, mut transitions) =
		agents.get_descendent_mut(ev.event_target())?;

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
	Ok(())
}

// /// Play animations for animators that load after the behavior starts
// pub(super) fn play_animation_on_load(
// 	parents: Query<&Parent>,
// 	mut loaded_animators: Query<
// 		(Entity, &mut AnimationPlayer, &mut AnimationTransitions),
// 		Added<AnimationPlayer>,
// 	>,
// 	query: Query<(&Running, &PlayAnimation)>,
// ) {
// 	for (entity, mut player, mut transitions) in loaded_animators.iter_mut() {
// 		let Some(play_animation) =
// 			parents.iter_ancestors_inclusive(entity).find_map(|parent| {
// 				query.iter().find_map(|(target, play_animation)| {
// 					if target.origin == parent {
// 						Some(play_animation)
// 					} else {
// 						None
// 					}
// 				})
// 			})
// 		else {
// 			continue;
// 		};
// 		if !player.is_playing_animation(play_animation.animation)
// 			|| play_animation.trigger_if_playing
// 		{
// 			transitions
// 				.play(
// 					&mut player,
// 					play_animation.animation,
// 					play_animation.transition_duration,
// 				)
// 				.set_repeat(play_animation.repeat);
// 		}
// 	}
// }

/// convenience system to create an [`AnimationPlayer`] from a clip
#[cfg(test)]
pub fn clip_to_player(
	clip: In<AnimationClip>,
	mut commands: Commands,
	mut animations: ResMut<Assets<AnimationClip>>,
	mut graphs: ResMut<Assets<AnimationGraph>>,
) -> (Entity, AnimationNodeIndex) {
	let (graph, animation_index) =
		AnimationGraph::from_clip(animations.add(clip.0));
	let player = AnimationPlayer::default();
	// player.play(animation_index).repeat();

	let player_entity = commands
		.spawn((
			AnimationGraphHandle(graphs.add(graph)),
			player,
			AnimationTransitions::new(),
		))
		.id();
	(player_entity, animation_index)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use bevy::animation::AnimationEvent;
	use sweet::prelude::*;

	#[derive(Clone, AnimationEvent)]
	struct MyEvent(u32);

	fn setup() -> (App, Store<Vec<u32>>, Entity, AnimationNodeIndex) {
		let store = Store::default();
		let mut app = App::new();
		app.add_plugins((AssetPlugin::default(), AnimationPlugin))
			.insert_time()
			.add_observer(move |foo: On<MyEvent>| {
				store.push(foo.0);
			})
			.run_once();

		let mut clip = AnimationClip::default();
		// animation.set_duration(2.0);

		clip.add_event(0.0, MyEvent(0));
		clip.add_event(1.0, MyEvent(1));
		clip.add_event(2.0, MyEvent(2));
		clip.add_event(3.0, MyEvent(3));
		let (entity, index) = app
			.world_mut()
			.run_system_cached_with(clip_to_player, clip)
			.unwrap();

		(app, store, entity, index)
	}

	#[test]
	fn animation_basics() {
		let (mut app, store, entity, index) = setup();
		app.world_mut()
			.entity_mut(entity)
			.get_mut::<AnimationPlayer>()
			.unwrap()
			.play(index)
			.repeat();
		store.get().xpect_empty();
		app.update();
		store.get().xpect_empty();
		app.update_with_millis(1);
		store.get().xpect_eq(vec![0]);
		app.update_with_secs(1);
		store.get().xpect_eq(vec![0, 1]);
		app.update_with_secs(1);
		store.get().xpect_eq(vec![0, 1, 2]);
		app.update_with_secs(1);
		store.get().xpect_eq(vec![0, 1, 2, 3, 0]);
	}

	#[test]
	fn works() {
		let (mut app, store, entity, index) = setup();

		app.world_mut()
			.spawn((ChildOf(entity), PlayAnimation::new(index)))
			.trigger_action(GetOutcome);
		store.get().xpect_empty();
		app.update();
		store.get().xpect_empty();
		app.update_with_millis(1);
		store.get().xpect_eq(vec![0]);
		app.update_with_secs(1);
		store.get().xpect_eq(vec![0, 1]);
		app.update_with_secs(1);
		store.get().xpect_eq(vec![0, 1, 2]);
		app.update_with_secs(1);
		store.get().xpect_eq(vec![0, 1, 2, 3]);
	}
}
