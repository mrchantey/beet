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
///   ([`Thread::enter`]) *before* running the behavior, and closes their
///   sessions once it ends, so every caller (the start verb, a test, a
///   serverless scene) gets a correct thread. It claims no action slot, so
///   the behavior keeps its own.
/// - a [`CallOnStart`], which is what calls that entry point when the ancestral
///   run starts. It gets the start request (for its [`ThreadEntryParams`])
///   without owning argv, and works identically under a launched entry, which
///   fires the same request shape.
///
/// The call is detached, so a finite loop ([`RepeatTimes`],
/// [`RepeatWhileFunctionCallOutput`]) completes and leaves the servers serving
/// the final transcript, while an endless [`Repeat`] simply never returns, its
/// session never closing.
/// Calling the **thread**, never an "agent", is the whole point: the `Sequence`
/// is the behavior and runs its actors in order.
///
/// A thread under it declaring a [`MountThreadStore`] continues the newest
/// thread in the mount's path, filled from the start request, so a restart
/// resumes; declaring none, it starts a thread every run. A thread built per
/// request rather than per entry is a [`ThreadRoute`].
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(CallOnStart)]
#[require(ActionOverload<Request, Outcome> = RunThread::entry())]
pub struct RunThread;

impl RunThread {
	/// The thread's entry point: enter every thread at or under the entity
	/// ([`Thread::enter`], with the request's [`ThreadEntryParams`]), run
	/// the behavior to completion, then close each session.
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
				let outcome = entity.call::<(), Outcome>(()).await;
				Self::close_all(&entity.world(), &threads, &outcome).await?;
				outcome
			},
		))
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

	/// Close every session `threads` opened with the behavior's `outcome`, an
	/// error closing as a fail, then wait for each thread's records to reach
	/// its store.
	pub(crate) async fn close_all(
		world: &AsyncWorld,
		threads: &[Entity],
		outcome: &Result<Outcome>,
	) -> Result {
		let outcome = match outcome {
			Ok(outcome) => *outcome,
			Err(_) => Outcome::FAIL,
		};
		for thread in threads {
			Thread::close(world, *thread, outcome).await?;
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

	/// Pump long enough for the entry -> adopt -> mount -> run -> sync chain.
	fn pump(app: &mut App, ticks: usize) {
		for _ in 0..ticks {
			app.update();
		}
	}

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
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
				TableStore::new(store.clone()),
				children![scene(system, agent)],
			))
			.trigger(|entity| StartRunning::new(entity, Request::default()));
		pump(&mut app, 120);
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
		ThreadStore::new(TableStore::new(store.clone()), RelPath::new(RECORDS))
			.threads()
			.await
			.unwrap()
			.len()
	}

	/// A `{MountThreadStore}` thread, run twice against the same store,
	/// continues the stored thread its path names rather than creating a
	/// duplicate. Regression for the deferred mount: the sync used to flush the
	/// fresh, un-adopted thread before adoption ran, growing the store to two
	/// threads on the second run.
	// runs a full app twice; the async flushes can exceed the 5s default under
	// parallel test load.
	#[beet_core::test(timeout_ms = 30000)]
	async fn reload_adopts_without_duplicating() {
		let store = BlobStore::temp();
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
	/// with no start notification still adopts before running.
	#[beet_core::test(timeout_ms = 30000)]
	async fn a_direct_call_adopts_before_running() {
		let store = BlobStore::temp();
		let system = ActorId::from_u128(3);
		let agent = ActorId::from_u128(4);

		// seed the store through a normal start: the system seed plus one reply
		run_once(&store, system, agent);
		stored_threads(&store).await.xpect_eq(1);

		// .. then re-run it by calling the entity itself. An un-adopted thread
		// would run its seed alone, ending on two posts rather than continuing the
		// stored conversation, its sessions aside.
		let mut app = app();
		let root = app
			.world_mut()
			.spawn((store.clone(), TableStore::new(store.clone())))
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
