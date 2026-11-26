use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Markup verb holding a live thread: its window is entered when the entry
/// starts, with the start request, and lives as long as the runtime, as the
/// thread's canonical window. This is how a thread a person sits in is held; a
/// thread run as a function, nobody typing, is a [`ThreadRoute`]. The words are
/// the model's, defined once in the [`table`](crate::table) module docs.
///
/// Spread it onto a thread's outer root, anywhere under the servers. The entry
/// root declares [`SweepDescendants`], so its start sweeps down to the verb:
///
/// ```rsx
/// <TuiServer {(CallOnReady, SweepDescendants)}>
///     <Router>
///         <Repeat {RunThread}>
///             <Thread {(Sequence, MountThreadStore{path:"threads/chat"})}> ..actors.. </Thread>
///         </Repeat>
///     </Router>
/// </TuiServer>
/// ```
///
/// It is two things, and neither is a boot mechanism of its own:
///
/// - the thread's **entry point**, an [`ActionOverload<Request, Outcome>`]
///   adapting the entity's behavior (its `Sequence`, or the loop wrapping it) to
///   a start request. Calling it enters every thread under it
///   ([`Thread::enter`]) *before* running the behavior, and flushes their
///   records once it ends, so every caller (the start verb, a test, a
///   serverless scene) gets a correct thread. It claims no action slot, so
///   the behavior keeps its own.
/// - a [`CallOnStart`], which is what calls that entry point when the ancestral
///   run starts. It gets the start request (for its [`ThreadEntryParams`])
///   without owning argv, and works identically under a launched entry, which
///   fires the same request shape.
///
/// Nothing awaits the call ([`CallOnStart`]), so a finite loop ([`RepeatTimes`],
/// [`RepeatWhileFunctionCallOutput`]) completes and leaves the servers serving
/// the final transcript, while an endless [`Repeat`] simply never returns.
/// Calling the **thread**, never an "agent", is the whole point: the `Sequence`
/// is the behavior and runs its actors in order.
///
/// Holding only windows with no behavior, ie a group chat, it enters them and
/// runs nothing; two windows onto one stored thread sit under one
/// `RunThread`, which enters them in order, so the second finds the thread the
/// first created.
///
/// A thread under it declaring a [`MountThreadStore`] continues the newest
/// thread in the mount's path, filled from the start request, so a restart
/// resumes; declaring none, it starts a thread every run.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CallOnStart)]
#[require(ActionOverload<Request, Outcome> = RunThread::entry())]
pub struct RunThread;

impl RunThread {
	/// The thread's entry point: enter every thread at or under the entity
	/// ([`Thread::enter`], with the request's [`ThreadEntryParams`]), run
	/// the behavior to completion, then flush each thread to its store.
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
				// the params ride the start request, never a global env read
				let entry = ThreadEntry::parse(cx.input.parts().clone())?;
				let threads = Self::enter_all(&entity, &entry).await?;
				// the canonical action, resolved by its own signature: an endless
				// `Repeat` never returns, a finite loop completes while the servers
				// stay parked.
				let outcome =
					match Self::holds_no_behavior(&entity, &threads).await? {
						true => Outcome::PASS.xok(),
						false => entity.call::<(), Outcome>(()).await,
					};
				Self::flush_all(&entity.world(), &threads).await?;
				outcome
			},
		))
	}

	/// Whether there is nothing to run: `entity` serves no `() -> Outcome`
	/// and no window of `threads` holds behavior, so people post whenever they
	/// like. An entity with no action over a window that does hold behavior is
	/// a mistake, left to the call to report.
	async fn holds_no_behavior(
		entity: &AsyncEntity,
		threads: &[Entity],
	) -> Result<bool> {
		let threads = threads.to_vec();
		entity
			.with_state::<(Query<&ActionMeta>, ThreadWindows), _>(
				move |entity, (metas, windows)| {
					!metas
						.get(entity)
						.is_ok_and(|meta| meta.matches::<(), Outcome>())
						&& !threads
							.iter()
							.any(|thread| windows.holds_behavior(*thread))
				},
			)
			.await
	}

	/// Enter every [`Thread`] at or under `root` with `entry`, in tree order
	/// ([`Thread::enter`]), answering them. Each entry reduces its thread
	/// first, ahead of the scheduled `First` reduce, so the behavior never
	/// receives raw, action-less `<CreateActor>` spans.
	pub(crate) async fn enter_all(
		root: &AsyncEntity,
		entry: &ThreadEntry,
	) -> Result<Vec<Entity>> {
		let world = root.world().clone();
		let threads = root
			.with_state::<(Query<&Children>, Query<(), With<Thread>>), _>(
				|root, (children, threads)| {
					children
						.iter_descendants_inclusive(root)
						.filter(|entity| threads.contains(*entity))
						.collect::<Vec<_>>()
				},
			)
			.await?;
		for thread in &threads {
			Thread::enter(world.clone(), *thread, entry.clone()).await?;
		}
		threads.xok()
	}

	/// Wait for the records of each of `threads` to reach its store
	/// ([`ThreadStore::flush`]). Nothing is posted: a session has no end to
	/// record.
	pub(crate) async fn flush_all(
		world: &AsyncWorld,
		threads: &[Entity],
	) -> Result {
		for thread in threads {
			ThreadStore::flush(&world.entity(*thread)).await?;
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

	/// The directory every scene keeps its thread records under.
	const RECORDS: &str = "threads";

	/// A persistent thread scene: pinned actor ids, one system seed and one
	/// mock agent. One agent turn, no loop, so the run finishes.
	fn scene(system: ActorId, agent: ActorId) -> impl Bundle {
		(
			Thread::default(),
			Sequence::new(),
			RunThread,
			MountThreadStore {
				path: RelPath::new(RECORDS),
				new: false,
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

	/// Pump long enough for the enter -> run -> flush chain.
	fn pump(app: &mut App, ticks: usize) {
		for _ in 0..ticks {
			app.update();
		}
	}

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		app
	}

	/// The start sweep declaration a real entry root authors, typed for the
	/// start verb's event.
	fn sweep() -> SweepDescendants<StartRunning<Request>> { default() }

	/// Run a persistent thread to completion through the start notification: a
	/// root holding the store, whose child is the thread, declaring the sweep as
	/// a real entry root does.
	fn run_once(store: &BlobStore, system: ActorId, agent: ActorId) {
		let mut app = app();
		app.world_mut()
			.spawn((
				sweep(),
				store.clone(),
				JsonTableStore::over(store.clone()).unwrap(),
				children![scene(system, agent)],
			))
			.trigger(|entity| StartRunning::new(entity, Request::default()));
		pump(&mut app, 120);
	}

	/// Started over two windows onto one path, neither holding behavior, it
	/// opens both onto one thread and runs nothing.
	#[beet_core::test]
	fn holds_two_windows_with_no_behavior() {
		let store = BlobStore::new(InMemoryBlobStore::new());
		let window = |id: u128, name: &str| {
			(
				Thread::default(),
				MountThreadStore {
					path: RelPath::new(RECORDS),
					new: false,
				},
				children![Actor::new_with_id(
					ActorId::from_u128(id),
					name,
					ActorKind::User,
				)],
			)
		};
		let mut app = app();
		app.world_mut()
			.spawn((
				sweep(),
				store.clone(),
				JsonTableStore::over(store.clone()).unwrap(),
				children![(RunThread, children![
					window(1, "Ann"),
					window(2, "Bob")
				])],
			))
			.trigger(|entity| StartRunning::new(entity, Request::default()));
		pump(&mut app, 120);
		let threads = app.world_mut().query_once::<(&Thread, &ThreadRecords)>();
		threads.len().xpect_eq(2);
		threads[0].0.id().xpect_eq(threads[1].0.id());
		for (_, records) in threads {
			records
				.posts()
				.iter()
				.map(Post::author)
				.collect::<Vec<_>>()
				.xpect_eq(vec![ActorId::from_u128(1), ActorId::from_u128(2)]);
		}
	}

	/// Call `entity` as `In -> Outcome` and pump the app until it settles.
	fn call<In: 'static + Send + Sync + Default>(
		app: &mut App,
		entity: Entity,
	) {
		app.world_mut().entity_mut(entity).run_async_local(
			|entity| async move {
				entity.call::<In, Outcome>(In::default()).await.map(|_| ())
			},
		);
		pump(app, 120);
	}

	/// The stored thread count, read through a fresh thread store so the
	/// records are re-read rather than served from an in-memory snapshot.
	async fn stored_threads(store: &BlobStore) -> usize {
		ThreadStore::new(
			JsonTableStore::over(store.clone()).unwrap(),
			RelPath::new(RECORDS),
		)
		.threads()
		.await
		.unwrap()
		.len()
	}

	/// A `{MountThreadStore}` thread, run twice against the same store,
	/// continues the stored thread its path names rather than creating a
	/// duplicate. Regression for the deferred mount: a flush ahead of the entry
	/// would write the thread it is about to leave for the stored one, growing
	/// the store to two threads on the second run.
	// runs a full app twice; the async flushes can exceed the 5s default under
	// parallel test load.
	#[beet_core::test(timeout_ms = 30000)]
	async fn reload_continues_without_duplicating() {
		let store = BlobStore::new(InMemoryBlobStore::new());
		let system = ActorId::from_u128(1);
		let agent = ActorId::from_u128(2);

		run_once(&store, system, agent);
		stored_threads(&store).await.xpect_eq(1);

		// reload: the same path continues the stored thread, no duplicate
		run_once(&store, system, agent);
		stored_threads(&store).await.xpect_eq(1);
	}

	/// Calling a mounted thread directly is the whole thread, not half of it: the
	/// entry point is the entity's own `Request -> Outcome` action, so a caller
	/// with no start notification still enters before running.
	#[beet_core::test(timeout_ms = 30000)]
	async fn a_direct_call_enters_before_running() {
		let store = BlobStore::new(InMemoryBlobStore::new());
		let system = ActorId::from_u128(3);
		let agent = ActorId::from_u128(4);

		// seed the store through a normal start: the system seed plus one reply
		run_once(&store, system, agent);
		stored_threads(&store).await.xpect_eq(1);

		// .. then re-run it by calling the entity itself. A thread not entered
		// would run its seed alone, ending on two posts rather than continuing the
		// stored conversation, its joins aside.
		let mut app = app();
		let root = app
			.world_mut()
			.spawn((
				store.clone(),
				JsonTableStore::over(store.clone()).unwrap(),
			))
			.id();
		let thread = app
			.world_mut()
			.spawn((ChildOf(root), scene(system, agent)))
			.id();
		call::<Request>(&mut app, thread);
		app.world()
			.get::<ThreadRecords>(thread)
			.unwrap()
			.posts()
			.iter()
			.filter(|post| post.intent() != PostIntent::Log)
			.count()
			.xpect_eq(3);
		stored_threads(&store).await.xpect_eq(1);
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
				.get::<ThreadRecords>(thread)
				.map(|records| records.posts().len())
				.unwrap_or_default()
		};
		(replies(&mut app, booted) > 1).xpect_true();
		// ... while the untouched entry never ran its own
		replies(&mut app, idle).xpect_eq(1);
	}
}
