use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait EpisodeParams: Debug + Clone + Reflect {
	fn num_episodes(&self) -> u32;
}

#[derive(Debug, Event)]
pub struct StartEpisode<T: EpisodeParams> {
	pub trainer: Entity,
	pub episode: u32,
	pub params: T,
}
#[derive(Debug, Event)]
pub struct EndEpisode<T: EpisodeParams> {
	pub trainer: Entity,
	phantom: PhantomData<T>,
}

impl EndEpisode<FrozenLakeEpParams> {
	pub fn new(trainer: Entity) -> Self {
		Self {
			trainer,
			phantom: PhantomData,
		}
	}
}

#[derive(Default)]
pub struct EpisodeRunnerPlugin<T: EpisodeParams> {
	phantom: PhantomData<T>,
}

impl<T: EpisodeParams> Plugin for EpisodeRunnerPlugin<T> {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(init_episode_runner::<T>, handle_episode_end::<T>).in_set(TickSet),
		)
		.add_event::<StartEpisode<T>>()
		.add_event::<EndEpisode<T>>();
	}
}

/// Adding this component to root entities spawned in an environment
/// will recursively despawn them when an episode ends.
#[derive(Debug, Clone, Copy, Component, Deref, Reflect)]
pub struct EpisodeOwner(pub Entity);

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct EpisodeRunner<T: EpisodeParams> {
	params: T,
	filename: Option<Cow<'static, str>>,
	episode: u32,
}

impl<T: EpisodeParams> EpisodeRunner<T> {
	pub fn new(params: T) -> Self {
		Self {
			params,
			filename: None,
			episode: 0,
		}
	}
	pub fn with_outfile(
		mut self,
		filename: impl Into<Cow<'static, str>>,
	) -> Self {
		self.filename = Some(filename.into());
		self
	}
}

pub fn init_episode_runner<T: EpisodeParams>(
	mut events: EventWriter<StartEpisode<T>>,
	runners: Query<(Entity, &mut EpisodeRunner<T>), Added<EpisodeRunner<T>>>,
) {
	for (entity, trainer) in runners.iter() {
		events.send(StartEpisode {
			trainer: entity,
			episode: trainer.episode,
			params: trainer.params.clone(),
		});
	}
}

pub fn handle_episode_end<T: EpisodeParams>(
	mut commands: Commands,
	mut start_events: EventWriter<StartEpisode<T>>,
	mut end_events: EventReader<EndEpisode<T>>,
	mut ep_entities: Query<(Entity, &EpisodeOwner)>,
	mut trainers: Query<(Entity, &mut EpisodeRunner<T>)>,
) {
	for event in end_events.read() {
		if let Ok((runner_entity, mut runner)) = trainers.get_mut(event.trainer)
		{
			for (ep_entity, parent_runner) in ep_entities.iter_mut() {
				if **parent_runner == runner_entity {
					commands.entity(ep_entity).despawn_recursive();
				}
			}
			runner.episode += 1;
			if runner.episode < runner.params.num_episodes() {
				start_events.send(StartEpisode {
					trainer: runner_entity,
					episode: runner.episode,
					params: runner.params.clone(),
				});
			} else {
				println!("Training complete");
				// todo!("Save model");
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	fn start_ep(
		mut commands: Commands,
		mut events: EventReader<StartEpisode<FrozenLakeEpParams>>,
	) {
		for event in events.read() {
			commands.spawn(EpisodeOwner(event.trainer));
		}
	}

	fn end_ep(
		mut events: EventWriter<EndEpisode<FrozenLakeEpParams>>,
		query: Query<&EpisodeOwner, Added<EpisodeOwner>>,
	) {
		for trainer in query.iter() {
			events.send(EndEpisode::new(**trainer));
		}
	}

	#[test]
	fn spawns_and_cleans_up_ep() -> Result<()> {
		let mut app = App::new();

		app.add_plugins((
			LifecyclePlugin::default(),
			EpisodeRunnerPlugin::<FrozenLakeEpParams>::default(),
		))
		.add_systems(Update, start_ep.in_set(PostTickSet))
		.add_systems(Update, end_ep.in_set(PreTickSet));
		let mut params = FrozenLakeEpParams::default();
		params.learn_params.n_training_episodes = 1;
		app.world_mut().spawn(EpisodeRunner::new(params));

		expect(app.world().entities().len()).to_be(1)?;

		app.update();
		expect(app.world().entities().len()).to_be(2)?;
		app.update();
		expect(app.world().entities().len()).to_be(1)?;

		Ok(())
	}
}
