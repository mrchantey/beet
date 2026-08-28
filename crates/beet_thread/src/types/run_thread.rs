use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Markup verb that runs a thread when the entry boots: it adopts the thread's
/// persistent store, then calls its behavior (the thread's `Sequence`, or the
/// loop wrapping it).
///
/// Spread it onto a thread's outer root, anywhere under the servers:
///
/// ```rsx
/// <TuiServer {CallOnReady}>
///     <Router>
///         <Repeat {RunThread}>
///             <Thread {Sequence}> ..actors.. </Thread>
///         </Repeat>
///     </Router>
/// </TuiServer>
/// ```
///
/// It owns no action slot and never writes [`AppExit`]: process lifetime belongs
/// to the servers. The kick rides the start notification a [`RunningSet`] fires
/// ([`StartRunning<Request>`], see [`RunThread::kick_on_boot`]), which is what
/// makes it both markup-friendly and boot-free: it gets the boot request (for
/// `--new`) without owning argv, and works identically under a launched entry, which
/// fires the same request shape.
///
/// The call is detached, so a finite loop ([`RepeatTimes`],
/// [`RepeatWhileFunctionCallOutput`]) completes and leaves the servers serving
/// the final transcript, while an endless [`Repeat`] simply never returns.
/// Calling the **thread**, never an "agent", is the whole point: the `Sequence`
/// is the behavior and runs its actors in order.
///
/// When the thread carries a [`ThreadStore`] (eg via `{MountThreadStore{path:..}}`)
/// the stored conversation is adopted by seed hash before the first turn, so a
/// reload resumes instead of re-replying; `--new` (carried on the boot request)
/// discards it and starts fresh.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RunThread;

impl RunThread {
	/// Run the thread rooted at `entity`: adopt its persistent store (honoring
	/// `--new` on `parts`), reduce the authored scene, then call its behavior to
	/// completion.
	///
	/// The seam a test or a serverless scene kicks through directly, with no boot
	/// event; the start notification is just the caller that supplies the request.
	pub async fn kick(
		world: AsyncWorld,
		entity: Entity,
		parts: RequestParts,
	) -> Result {
		// `--new` rides the boot request, never a global env read mid-load.
		Self::adopt_store(
			world.clone(),
			entity,
			parts.get_param("new").is_some(),
		)
		.await?;
		// Reduce the authored scene into its `ThreadWindow` + behavior scene before
		// running it. The kick drives the behavior directly, ahead of the scheduled
		// `First` reduce, so without this the `Sequence` would receive raw, action-less
		// `<CreateActor>` spans and fail. Idempotent (`Without<ThreadWindow>`), so the
		// store path (already reduced in `Thread::adopt`) is unaffected.
		world
			.with(|world: &mut World| ThreadWindow::reduce_now(world))
			.await;
		// run the thread behavior: an endless `Repeat` never returns, a finite loop
		// completes while the servers stay parked.
		world.entity(entity).call::<(), Outcome>(()).await?;
		Ok(())
	}

	/// Adopt the persistent store of the thread (at or under `root`) declaring a
	/// [`MountThreadStore`], by seed hash, then mount it. The store is mounted
	/// *after* adoption so the persistence sync never flushes a fresh, un-adopted
	/// thread (which would fork a duplicate on every reload). `new` discards the
	/// stored conversation first. Ephemeral threads declare no store and skip this.
	async fn adopt_store(world: AsyncWorld, root: Entity, new: bool) -> Result {
		let Some((thread, mount)) = world
			.with(move |world: &mut World| Self::pending_store(world, root))
			.await
		else {
			return Ok(());
		};
		let store = mount.build();
		// `--new` discards the stored conversation and starts fresh
		if new {
			store.store_remove().await.ok();
		}
		Thread::adopt(world.clone(), store.clone(), thread).await?;
		world
			.with(move |world: &mut World| {
				if let Ok(mut thread) = world.get_entity_mut(thread) {
					thread.insert(store).remove::<MountThreadStore>();
				}
			})
			.await;
		Ok(())
	}

	/// The thread (at or under `root`) declaring a [`MountThreadStore`], with that
	/// declaration, if any: the persistence seam [`RunThread::adopt_store`] builds
	/// and adopts before mounting the store and kicking.
	fn pending_store(
		world: &mut World,
		root: Entity,
	) -> Option<(Entity, MountThreadStore)> {
		world.with_state::<(Query<&Children>, Query<&MountThreadStore>), _>(
			|(children, mounts)| {
				children
					.iter_descendants_inclusive(root)
					.find_map(|entity| {
						mounts
							.get(entity)
							.ok()
							.map(|mount| (entity, mount.clone()))
					})
			},
		)
	}

	/// Observer: kick every [`RunThread`] at or under a booting server root, once
	/// per boot.
	///
	/// Global rather than per-entity so a `RunThread` needs no boot machinery of
	/// its own; the ancestry filter is what keeps co-resident entries from kicking
	/// each other's threads. Each kick is detached, so a parked chat never holds
	/// the fan-out up.
	pub(crate) fn kick_on_boot(
		ev: On<StartRunning<Request>>,
		children: Query<&Children>,
		threads: Query<(), With<RunThread>>,
		mut commands: Commands,
	) -> Result {
		let parts = ev.with(|request| request.parts().clone())?;
		for entity in children
			.iter_descendants_inclusive(ev.entity)
			.filter(|entity| threads.contains(*entity))
		{
			let parts = parts.clone();
			commands.entity(entity).queue_async_local(move |entity| {
				RunThread::kick(entity.world().clone(), entity.id(), parts)
			});
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A persistent thread scene: pinned actor ids so the seed hash is stable
	/// across runs, one system seed and one mock agent. One agent turn, no loop,
	/// so the run finishes.
	fn scene(path: &str, system: ActorId, agent: ActorId) -> impl Bundle {
		(
			Thread::default(),
			Sequence::new(),
			RunThread,
			MountThreadStore {
				path: path.to_string(),
			},
			children![
				(
					Actor::new_with_id(system, "System", ActorKind::System),
					children![Post::spawn("be brief")],
				),
				(
					Actor::new_with_id(agent, "Agent", ActorKind::Agent),
					MockPostStreamer::default(),
				),
			],
		)
	}

	/// Run a persistent thread to completion through the start notification: a
	/// root whose child is the thread, notified as a real start does.
	fn run_once(path: &str, system: ActorId, agent: ActorId) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		app.world_mut()
			.spawn(children![scene(path, system, agent)])
			.trigger(|entity| StartRunning::new(entity, Request::default()));
		// pump long enough for the async kick -> adopt -> mount -> run -> sync
		for _ in 0..120 {
			app.update();
		}
	}

	/// The stored thread count, read through a fresh store so the on-disk file is
	/// re-read rather than served from an in-memory snapshot.
	async fn stored_threads(path: &str) -> usize {
		BlobThreadStore::new(BlobStore::new(FsStore::new(WsPathBuf::new(path))))
			.threads()
			.await
			.unwrap()
			.len()
	}

	/// A `{MountThreadStore}` thread, run twice against the same store, adopts the
	/// stored thread on reload rather than forking a duplicate. Regression for the
	/// deferred mount: the sync used to flush the fresh, un-adopted thread before
	/// adoption ran, growing the store to two threads on the second run.
	// runs a full app twice against real disk; the async fs flushes can exceed
	// the 5s default under parallel test load.
	#[beet_core::test(timeout_ms = 30000)]
	async fn reload_adopts_without_duplicating() {
		let path = "target/tests/beet_thread/run-thread-reload";
		// stable ids so the seed hash matches across runs
		let system = ActorId::from_u128(1);
		let agent = ActorId::from_u128(2);
		BlobThreadStore::new(BlobStore::new(FsStore::new(WsPathBuf::new(
			path,
		))))
		.store_remove()
		.await
		.ok();

		run_once(path, system, agent);
		stored_threads(path).await.xpect_eq(1);

		// reload: the same seed hash adopts the stored thread, no duplicate
		run_once(path, system, agent);
		stored_threads(path).await.xpect_eq(1);
	}

	/// The kick is scoped by ancestry: booting one entry runs its own thread and
	/// leaves a co-resident entry's thread alone.
	#[beet_core::test]
	async fn kicks_only_its_own_entry() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		let ephemeral = || {
			(Thread::default(), Sequence::new(), RunThread, children![
				(Actor::user(), children![Post::spawn("hello")]),
				(Actor::agent(), MockPostStreamer::default()),
			])
		};
		let booted = app.world_mut().spawn(children![ephemeral()]).flush();
		let idle = app.world_mut().spawn(children![ephemeral()]).flush();
		app.world_mut()
			.entity_mut(booted)
			.trigger(|entity| StartRunning::new(entity, Request::default()));
		for _ in 0..60 {
			app.update();
		}

		// the booted entry's thread replied ...
		let replies = |app: &mut App, root: Entity| {
			let thread = app.world().entity(root).get::<Children>().unwrap()[0];
			app.world()
				.get::<ThreadWindow>(thread)
				.map(|window| window.posts().len())
				.unwrap_or_default()
		};
		(replies(&mut app, booted) > 1).xpect_true();
		// ... while the untouched entry never ran its own
		replies(&mut app, idle).xpect_eq(1);
	}
}
