use crate::PostTickSet;
use crate::PreTickSet;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;
use std::marker::PhantomData;

/// Emitted at the start of each episode within a session.
#[derive(Debug, Message)]
pub struct StartEpisode<T: EpisodeParams> {
	/// The session entity.
	pub session: Entity,
	/// The episode index (0-based).
	pub episode: u32,
	/// Session parameters.
	pub params: T,
}

/// Emitted once when a session is first added.
#[derive(Debug, Message)]
pub struct StartSession<T: EpisodeParams> {
	/// The session entity.
	pub session: Entity,
	/// Session parameters.
	pub params: T,
}

/// Emitted when the configured number of episodes have all completed.
#[derive(Debug, Message)]
pub struct EndSession<T: EpisodeParams> {
	/// The session entity.
	pub session: Entity,
	/// Session parameters.
	pub params: T,
}

/// Queue this to tell the session machinery a particular episode finished.
#[derive(Debug, Message)]
pub struct EndEpisode<T: EpisodeParams> {
	/// The session entity.
	pub session: Entity,
	phantom: PhantomData<T>,
}

impl<T: EpisodeParams> EndEpisode<T> {
	/// Create an [`EndEpisode`] for the given session.
	pub fn new(session: Entity) -> Self {
		Self {
			session,
			phantom: PhantomData,
		}
	}
}

/// Wires the message types and per-tick systems for one
/// [`EpisodeParams`] instantiation.
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

/// Pointer from a transient (agent/episode) entity to its owning session.
#[derive(Debug, Clone, Copy, Component, Deref, Reflect)]
pub struct SessionEntity(pub Entity);

/// Marker: despawn this entity when its parent session ends.
#[derive(Debug, Clone, Copy, Component, Reflect)]
pub struct DespawnOnSessionEnd;

/// Marker: despawn this entity when the current episode ends.
#[derive(Debug, Clone, Copy, Component, Reflect)]
pub struct DespawnOnEpisodeEnd;

/// Session component, holding parameters, progress, and optional save path.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct RlSession<T: EpisodeParams> {
	params: T,
	filename: Option<Cow<'static, str>>,
	episode: u32,
}

impl<T: EpisodeParams> RlSession<T> {
	/// Create a session from the given parameters.
	pub fn new(params: T) -> Self {
		Self {
			params,
			filename: None,
			episode: 0,
		}
	}

	/// Attach a filename used by callers to save the trained policy when
	/// the session ends.
	pub fn with_outfile(
		mut self,
		filename: impl Into<Cow<'static, str>>,
	) -> Self {
		self.filename = Some(filename.into());
		self
	}
}

/// On session-spawn, emits [`StartSession`] and the first [`StartEpisode`].
pub fn start_session<T: EpisodeParams>(
	mut start_session: MessageWriter<StartSession<T>>,
	mut start_episode: MessageWriter<StartEpisode<T>>,
	sessions: Query<(Entity, &RlSession<T>), Added<RlSession<T>>>,
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

/// Consumes [`EndEpisode`], advances the episode index, despawns entities
/// tagged [`DespawnOnEpisodeEnd`], and either queues the next episode or
/// ends the session.
pub fn handle_episode_end<T: EpisodeParams>(
	mut commands: Commands,
	mut start_ep: MessageWriter<StartEpisode<T>>,
	mut end_ep: MessageReader<EndEpisode<T>>,
	mut end_session: MessageWriter<EndSession<T>>,
	despawn_on_episode_end: Query<
		(Entity, &SessionEntity),
		With<DespawnOnEpisodeEnd>,
	>,
	despawn_on_session_end: Query<
		(Entity, &SessionEntity),
		With<DespawnOnSessionEnd>,
	>,
	mut sessions: Query<(Entity, &mut RlSession<T>)>,
) {
	for event in end_ep.read() {
		let Ok((session_entity, mut session)) = sessions.get_mut(event.session)
		else {
			continue;
		};
		// 1. cancel any running actions on the episode-scoped entities,
		//    then despawn. Without InterruptRun, a Repeat or other control
		//    flow node mid-iteration would try to call into the despawned
		//    subtree on the next tick and panic.
		for (e_despawn, parent_session) in despawn_on_episode_end.iter() {
			if **parent_session == session_entity {
				commands
					.entity(e_despawn)
					.queue(InterruptRun::<Outcome>::new())
					.despawn();
			}
		}
		session.episode += 1;
		// 2. either advance to the next episode or end the session
		if session.episode < session.params.num_episodes() {
			log::info!("Starting episode {}", session.episode);
			start_ep.write(StartEpisode {
				session: session_entity,
				episode: session.episode,
				params: session.params.clone(),
			});
		} else {
			log::info!("Ending session");
			end_session.write(EndSession {
				session: session_entity,
				params: session.params.clone(),
			});
			for (e_despawn, parent_session) in despawn_on_session_end.iter() {
				if **parent_session == session_entity {
					commands
						.entity(e_despawn)
						.queue(InterruptRun::<Outcome>::new())
						.despawn();
				}
			}
		}
	}
}
