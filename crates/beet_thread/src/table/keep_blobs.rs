//! How many files an agent's context carries, see [`KeepBlobs`].
use crate::prelude::*;
use beet_core::prelude::*;

/// On an agent: its context carries the bytes of the newest `count` files,
/// of any media type, and lists each older one as a line naming its post,
/// media type, name and size ([`PostFile::listing`]). Without it an agent
/// carries every file; `count:0` is references alone.
///
/// Nothing is rewritten to shrink a request: the records keep every post,
/// and a listed file is carried again once a tool reads it back, its answer
/// then the newest file. A window holds in memory only the bytes its agents'
/// contexts carry, so a camera loop stays bounded however long it runs.
///
/// ```rsx
/// <CreateActor name="Robot" kind="Agent" {(ModelStreamer{provider:OpenAi}, KeepBlobs{count:2})}/>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct KeepBlobs {
	/// How many of the newest files the context carries.
	pub count: usize,
}

impl KeepBlobs {
	pub fn new(count: usize) -> Self { Self { count } }
}

/// Drop the bytes each changed window holds in memory that none of its
/// agents' contexts carry: beyond the most any agent keeps, nothing for a
/// window with no agent, and, where a blob store holds the thread's bytes,
/// only those already handed to a flush. A window with an agent carrying
/// every file keeps them all.
pub(crate) fn release_bytes(
	mut windows: Query<
		(
			Entity,
			&mut ThreadRecords,
			Option<&ThreadStore>,
			Option<&SyncedRecords>,
		),
		Changed<ThreadRecords>,
	>,
	ancestors: Query<&ChildOf>,
	threads: Query<(), With<Thread>>,
	agents: Query<(Entity, &ActorRef, Option<&KeepBlobs>)>,
) {
	for (window, mut records, store, synced) in windows.iter_mut() {
		if records.held_bytes() == 0 {
			continue;
		}
		// the agents whose nearest window is this one
		let keep = agents
			.iter()
			.filter(|(agent, ..)| {
				ancestors
					.iter_ancestors_inclusive(*agent)
					.find(|ancestor| threads.contains(*ancestor))
					== Some(window)
			})
			.filter(|(_, actor, _)| {
				records
					.actor(actor)
					.is_ok_and(|actor| actor.kind() == ActorKind::Agent)
			})
			.map(|(.., keep)| keep.map_or(usize::MAX, |keep| keep.count))
			.max()
			.unwrap_or(0);
		let stored = store.is_some_and(|store| store.blobs().is_some());
		records.bypass_change_detection().release_bytes(keep, |id| {
			!stored || synced.is_some_and(|synced| synced.contains(id))
		});
	}
}

#[cfg(all(test, feature = "agent"))]
mod test {
	use crate::prelude::*;
	use crate::streaming::request_test;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A camera posts a photo a turn to a robot carrying two: over many
	/// turns its requests carry two images and list the rest by post, and
	/// the window holds two photos' bytes, whether a blob store keeps them
	/// all or nothing does.
	async fn camera_loop(stored: bool) {
		const TURNS: usize = 12;
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let blobs = BlobStore::temp();
		let root = app
			.world_mut()
			.spawn((TableStore::new(InMemoryTableStore::new()), blobs.clone()))
			.id();
		let mut window = app.world_mut().spawn((
			ChildOf(root),
			Thread::default(),
			children![
				Actor::new("Camera", ActorKind::User).unwrap(),
				(
					Actor::new("Robot", ActorKind::Agent).unwrap(),
					MockPostStreamer::default(),
					KeepBlobs::new(2),
				),
			],
		));
		if stored {
			window.insert(MountThreadStore {
				path: RelPath::new("threads"),
				new: false,
			});
		}
		let window = window.id();
		match stored {
			true => request_test::enter(&mut app, window).await,
			false => app.update(),
		}
		let robot = request_test::actor_entity(
			&mut app,
			&ActorId::from_static("robot"),
		);

		for turn in 0..TURNS {
			let mut entity = app.world_mut().entity_mut(window);
			let thread = entity.get::<Thread>().unwrap().id();
			entity.get_mut::<ThreadRecords>().unwrap().upsert_post(
				AgentPost::new_media(
					ActorId::from_static("camera"),
					thread,
					MediaType::Png,
					vec![turn as u8; 1024],
					None,
					PostStatus::Completed,
				),
			);
			app.update();
			let records = app.world().get::<ThreadRecords>(window).unwrap();
			records.held_bytes().xpect_less_or_equal_to(2);
			for request in request_test::requests(&mut app, robot).await {
				let text = request.to_json_string();
				text.matches("data:image/png")
					.count()
					.xpect_eq((turn + 1).min(2));
				text.matches(" in post ")
					.count()
					.xpect_eq((turn + 1).saturating_sub(2));
			}
		}
		if stored {
			app.world_mut()
				.run_async_local_then(async move |world: AsyncWorld| {
					ThreadStore::flush(&world.entity(window)).await
				})
				.await
				.unwrap();
		}
		// the store keeps every photo, nothing else does
		blobs.get_all().await.unwrap().len().xpect_eq(if stored {
			TURNS
		} else {
			0
		});
	}

	#[beet_core::test]
	async fn a_camera_loop_stays_bounded() { camera_loop(false).await; }

	#[beet_core::test]
	async fn a_stored_camera_loop_stays_bounded() { camera_loop(true).await; }
}
