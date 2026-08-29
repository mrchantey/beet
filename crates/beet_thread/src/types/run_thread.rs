use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Markup verb that makes an entity run a thread, and runs it when the entry
/// starts.
///
/// Spread it onto a thread's outer root, anywhere under the servers. The entry
/// root declares [`SweepDescendants`], so its start sweeps down to the verb:
///
/// ```rsx
/// <TuiServer {(CallOnReady, SweepDescendants)}>
///     <Router>
///         <Repeat {RunThread}>
///             <Thread {Sequence}> ..actors.. </Thread>
///         </Repeat>
///     </Router>
/// </TuiServer>
/// ```
///
/// It is two things, and neither is a boot mechanism of its own:
///
/// - the thread's **entry point**, an [`ActionOverload<Request, Outcome>`]
///   adapting the entity's behavior (its `Sequence`, or the loop wrapping it) to
///   a start request. Calling it adopts the thread's persistent store and reduces
///   the authored scene *before* running the behavior, so every caller — the
///   start verb, a test, a serverless scene — gets a correct thread. It claims no
///   action slot, so the behavior keeps its own.
/// - a [`CallOnStart`], which is what calls that entry point when the ancestral
///   run starts. It gets the start request (for `--new`) without owning argv, and
///   works identically under a launched entry, which fires the same request shape.
///
/// The call is detached, so a finite loop ([`RepeatTimes`],
/// [`RepeatWhileFunctionCallOutput`]) completes and leaves the servers serving
/// the final transcript, while an endless [`Repeat`] simply never returns.
/// Calling the **thread**, never an "agent", is the whole point: the `Sequence`
/// is the behavior and runs its actors in order.
///
/// When the thread carries a [`ThreadStore`] (eg via `{MountThreadStore{path:..}}`)
/// the stored conversation is adopted by seed hash before the first turn, so a
/// reload resumes instead of re-replying; `--new` (carried on the start request)
/// discards it and starts fresh.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CallOnStart)]
#[require(ActionOverload<Request, Outcome> = RunThread::entry())]
pub struct RunThread;

impl RunThread {
	/// The thread's entry point: adopt its persistent store (honoring `--new` on
	/// the request), reduce the authored scene, then run the behavior to
	/// completion.
	///
	/// An [`ActionOverload`] rather than a free function, so the entity's
	/// [`ActionMeta`] advertises the whole thread as `Request -> Outcome` and no
	/// caller can enter halfway. The canonical `() -> Outcome` stays what it always
	/// was, the bare behavior, which is what this delegates to once the thread is
	/// ready.
	fn entry() -> ActionOverload<Request, Outcome> {
		ActionOverload::new(Action::new_async(
			async |cx: ActionContext<Request>| -> Result<Outcome> {
				let entity = cx.caller.clone();
				let world = entity.world().clone();
				// `--new` rides the start request, never a global env read mid-load.
				Self::adopt_store(
					world.clone(),
					entity.id(),
					cx.input.has_param("new"),
				)
				.await?;
				// Reduce the authored scene into its `ThreadWindow` + behavior scene
				// before running it. The entry drives the behavior directly, ahead of
				// the scheduled `First` reduce, so without this the `Sequence` would
				// receive raw, action-less `<CreateActor>` spans and fail. Idempotent
				// (`Without<ThreadWindow>`), so the store path (already reduced in
				// `Thread::adopt`) is unaffected.
				world
					.with(|world: &mut World| ThreadWindow::reduce_now(world))
					.await;
				// the canonical action, resolved by its own signature: an endless
				// `Repeat` never returns, a finite loop completes while the servers
				// stay parked.
				entity.call::<(), Outcome>(()).await
			},
		))
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
	/// and adopts before mounting the store and running the behavior.
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

	/// Pump long enough for the entry -> adopt -> mount -> run -> sync chain.
	fn pump(app: &mut App, ticks: usize) {
		for _ in 0..ticks {
			app.update();
		}
	}

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		app
	}

	/// The start sweep declaration a real entry root authors, typed for the
	/// start verb's event.
	fn sweep() -> SweepDescendants<StartRunning<Request>> { default() }

	/// Run a persistent thread to completion through the start notification: a
	/// root whose child is the thread, declaring the sweep as a real entry root
	/// does.
	fn run_once(path: &str, system: ActorId, agent: ActorId) {
		let mut app = app();
		app.world_mut()
			.spawn((sweep(), children![scene(path, system, agent)]))
			.trigger(|entity| StartRunning::new(entity, Request::default()));
		pump(&mut app, 120);
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

	/// Clear whatever a previous run left at `path`.
	async fn clear(path: &str) {
		BlobThreadStore::new(BlobStore::new(FsStore::new(WsPathBuf::new(
			path,
		))))
		.store_remove()
		.await
		.ok();
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
		clear(path).await;

		run_once(path, system, agent);
		stored_threads(path).await.xpect_eq(1);

		// reload: the same seed hash adopts the stored thread, no duplicate
		run_once(path, system, agent);
		stored_threads(path).await.xpect_eq(1);
	}

	/// Calling a mounted thread directly is the whole thread, not half of it: the
	/// entry point is the entity's own `Request -> Outcome` action, so a caller
	/// with no start notification still adopts before running.
	#[beet_core::test(timeout_ms = 30000)]
	async fn a_direct_call_adopts_before_running() {
		let path = "target/tests/beet_thread/run-thread-direct-call";
		let system = ActorId::from_u128(3);
		let agent = ActorId::from_u128(4);
		clear(path).await;

		// seed the store through a normal start: the system seed plus one reply
		run_once(path, system, agent);
		stored_threads(path).await.xpect_eq(1);

		// .. then re-run it by calling the entity itself. An un-adopted thread
		// would run its seed alone, ending on two posts rather than continuing the
		// stored conversation.
		let mut app = app();
		let thread = app.world_mut().spawn(scene(path, system, agent)).id();
		app.world_mut().entity_mut(thread).run_async_local(
			|thread| async move {
				thread
					.call::<Request, Outcome>(Request::default())
					.await
					.map(|_| ())
			},
		);
		pump(&mut app, 120);
		app.world()
			.get::<ThreadWindow>(thread)
			.unwrap()
			.posts()
			.len()
			.xpect_eq(3);
		stored_threads(path).await.xpect_eq(1);
	}

	/// The start sweep never leaves its root's subtree: starting one entry runs
	/// its own thread and leaves a co-resident entry's thread alone.
	#[beet_core::test]
	async fn kicks_only_its_own_entry() {
		let mut app = app();
		let ephemeral = || {
			(Thread::default(), Sequence::new(), RunThread, children![
				(Actor::user(), children![Post::spawn("hello")]),
				(Actor::agent(), MockPostStreamer::default()),
			])
		};
		let booted = app
			.world_mut()
			.spawn((sweep(), children![ephemeral()]))
			.flush();
		let idle = app
			.world_mut()
			.spawn((sweep(), children![ephemeral()]))
			.flush();
		app.world_mut()
			.entity_mut(booted)
			.trigger(|entity| StartRunning::new(entity, Request::default()));
		pump(&mut app, 60);

		// the started entry's thread replied ...
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
