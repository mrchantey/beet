use beet_core::prelude::*;
use beet_flow::prelude::*;
use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait EpisodeParams: Debug + Clone + Reflect {
	fn num_episodes(&self) -> u32;
}

#[derive(Debug, Message)]
pub struct StartEpisode<T: EpisodeParams> {
	pub session: Entity,
	pub episode: u32,
	pub params: T,
}
#[derive(Debug, Message)]
pub struct StartSession<T: EpisodeParams> {
	pub session: Entity,
	pub params: T,
}
#[derive(Debug, Message)]
pub struct EndSession<T: EpisodeParams> {
	pub session: Entity,
	pub params: T,
}

#[derive(Debug, Message)]
pub struct EndEpisode<T: EpisodeParams> {
	pub session: Entity,
	phantom: PhantomData<T>,
}

impl<T: EpisodeParams> EndEpisode<T> {
	pub fn new(session: Entity) -> Self {
		Self {
			session,
			phantom: PhantomData,
		}
	}
}

#[derive(Default)]
pub struct RlSessionPlugin<T: EpisodeParams> {
	phantom: PhantomData<T>,
}

impl<T: EpisodeParams> Plugin for RlSessionPlugin<T> {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				start_session::<T>.in_set(PreTickSet),
				handle_episode_end::<T>.in_set(PostTickSet),
			),
		)
		.add_message::<StartSession<T>>()
		.add_message::<EndSession<T>>()
		.add_message::<StartEpisode<T>>()
		.add_message::<EndEpisode<T>>();
	}
}

/// Pointer to the entity that owns this entity
#[derive(Debug, Clone, Copy, Component, Deref, Reflect)]
pub struct SessionEntity(pub Entity);

#[derive(Debug, Clone, Copy, Component, Reflect)]
pub struct DespawnOnSessionEnd;
#[derive(Debug, Clone, Copy, Component, Reflect)]
pub struct DespawnOnEpisodeEnd;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct RlSession<T: EpisodeParams> {
	params: T,
	filename: Option<Cow<'static, str>>,
	episode: u32,
}

impl<T: EpisodeParams> RlSession<T> {
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

pub fn start_session<T: EpisodeParams>(
	mut start_session: MessageWriter<StartSession<T>>,
	mut start_episode: MessageWriter<StartEpisode<T>>,
	sessions: Query<(Entity, &mut RlSession<T>), Added<RlSession<T>>>,
) {
	for (entity, session) in sessions.iter() {
		start_session.write(StartSession {
			session: entity,
			params: session.params.clone(),
		});
		start_episode.write(StartEpisode {
			session: entity,
			episode: session.episode,
			params: session.params.clone(),
		});
	}
}

pub fn handle_episode_end<T: EpisodeParams>(
	mut commands: Commands,
	mut start_ep: MessageWriter<StartEpisode<T>>,
	mut end_ep: MessageReader<EndEpisode<T>>,
	mut end_session: MessageWriter<EndSession<T>>,
	mut despawn_on_episode_end: Query<
		(Entity, &SessionEntity),
		With<DespawnOnEpisodeEnd>,
	>,
	mut despawn_on_session_end: Query<
		(Entity, &SessionEntity),
		With<DespawnOnSessionEnd>,
	>,
	mut sessions: Query<(Entity, &mut RlSession<T>)>,
) {
	for event in end_ep.read() {
		if let Ok((session_entity, mut session)) =
			sessions.get_mut(event.session)
		{
			for (e_despawn, parent_session) in despawn_on_episode_end.iter_mut()
			{
				if **parent_session == session_entity {
					commands.entity(e_despawn).despawn();
				}
			}
			session.episode += 1;
			if session.episode < session.params.num_episodes() {
				log::info!("Starting episode {}", session.episode);
				start_ep.write(StartEpisode {
					session: session_entity,
					episode: session.episode,
					params: session.params.clone(),
				});
			} else {
				log::info!("Ending session");
				// complete!
				end_session.write(EndSession {
					session: session_entity,
					params: session.params.clone(),
				});

				for (e_despawn, parent_session) in
					despawn_on_session_end.iter_mut()
				{
					if **parent_session == session_entity {
						commands.entity(e_despawn).despawn();
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;

	fn start_ep(
		mut commands: Commands,
		mut events: MessageReader<StartEpisode<FrozenLakeEpParams>>,
	) {
		for event in events.read() {
			commands.spawn((SessionEntity(event.session), DespawnOnEpisodeEnd));
		}
	}

	fn end_ep(
		mut events: MessageWriter<EndEpisode<FrozenLakeEpParams>>,
		query: Query<&SessionEntity, Added<SessionEntity>>,
	) {
		for trainer in query.iter() {
			events.write(EndEpisode::new(**trainer));
		}
	}

	#[test]
	#[ignore = "todo failing not sure why"]
	fn spawns_and_cleans_up_ep() {
		let mut app = App::new();

		app.add_plugins((
			MinimalPlugins,
			ControlFlowPlugin::default(),
			RlSessionPlugin::<FrozenLakeEpParams>::default(),
		))
		.add_systems(Update, (start_ep, end_ep).in_set(TickSet));
		let mut params = FrozenLakeEpParams::default();
		params.learn_params.n_training_episodes = 1;
		let len = app.world().entities().len();
		app.world_mut().spawn(RlSession::new(params));
		app.world().entities().len().xpect_eq(len + 1);
		app.update();
		app.world().entities().len().xpect_eq(len + 2);
		app.update();
		app.world().entities().len().xpect_eq(len + 1);
	}
}
