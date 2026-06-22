use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Markup verb that runs a thread as a *program*: on load it calls this entity's
/// behavior (the thread's `Sequence`, or the loop wrapping it) and holds the
/// process open for the duration of the run.
///
/// Spread it onto a thread's outer root so loading and spawning the scene starts
/// the turn loop, with no hand-written kick glue:
///
/// ```rsx
/// <div {(Repeat, RunThread)}>
///     <div {Thread} {Sequence}> ..actors.. </div>
/// </div>
/// ```
///
/// It is *not* a plain [`CallOnSpawn`]: calling the thread is async and can take
/// the whole process lifetime, but the `beet` binary releases its load-scope
/// [`KeepAlive`] claim the moment the entry finishes loading. So `RunThread`
/// takes its own claim in `on_add` (synchronously, during load, before that
/// release) and drops it when the run returns. An endless `Repeat` never returns,
/// so an interactive or auto chat runs until Ctrl+C; a finite loop
/// (`RepeatTimes`, `RepeatWhileFunctionCallOutput`) drops the claim on
/// completion, and when the last claim goes the binary exits cleanly. Calling the
/// **thread**, never an "agent", is the whole point: the `Sequence` is the
/// behavior and runs its actors in order.
///
/// When the thread carries a [`ThreadStore`] (eg via `{MountThreadStore{path:..}}`)
/// the stored conversation is adopted by seed hash before the first turn, so a
/// reload resumes instead of re-replying; `--new` discards it and starts fresh.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = run_thread_on_add)]
pub struct RunThread;

/// Take the process-lifetime claim during load, so the program does not exit
/// before its first turn (the binary drops its own load-scope claim once the
/// entry is built). A no-op where no [`KeepAlive`] resource exists (eg headless
/// tests that pump frames manually).
fn run_thread_on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).insert(KeepAliveGuard);
}

/// Call every freshly-spawned [`RunThread`] root, then release its [`KeepAlive`]
/// claim once the run returns. A persistent thread is adopted by seed hash first.
pub fn run_thread_on_load(
	async_commands: AsyncCommands,
	query: Query<Entity, Added<RunThread>>,
) {
	for entity in query.iter() {
		async_commands.run(async move |world: AsyncWorld| -> Result {
			// a persistent thread declares `MountThreadStore`: build its store,
			// adopt the stored conversation by seed hash, then mount the store. The
			// store is mounted *after* adoption so the sync never flushes a fresh,
			// un-adopted thread (which would fork a duplicate on every reload).
			// Ephemeral threads declare none and skip this.
			if let Some((thread, mount)) = world
				.with(move |world: &mut World| pending_store(world, entity))
				.await
			{
				let store = mount.build();
				// `--new` discards the stored conversation and starts fresh
				if CliArgs::parse_env().params.contains_key("new") {
					store.store_remove().await.ok();
				}
				adopt_thread(world.clone(), store.clone(), thread).await?;
				world
					.with(move |world: &mut World| {
						if let Ok(mut thread) = world.get_entity_mut(thread) {
							thread.insert(store).remove::<MountThreadStore>();
						}
					})
					.await;
			}
			let result = world.entity(entity).call::<(), Outcome>(()).await;
			// drop the process-lifetime claim now the run is over; the binary exits
			// when the last claim goes. Done regardless of the run's outcome.
			world
				.with(move |world: &mut World| {
					if let Ok(mut entity) = world.get_entity_mut(entity) {
						entity.remove::<KeepAliveGuard>();
					}
				})
				.await;
			result?;
			Ok(())
		});
	}
}

/// The thread (at or under `root`) declaring a [`MountThreadStore`], with that
/// declaration, if any: the persistence seam [`run_thread_on_load`] builds and
/// adopts before mounting the store and kicking.
fn pending_store(
	world: &mut World,
	root: Entity,
) -> Option<(Entity, MountThreadStore)> {
	world.with_state::<(Query<&Children>, Query<&MountThreadStore>), _>(
		|(children, mounts)| {
			std::iter::once(root)
				.chain(children.iter_descendants(root))
				.find_map(|entity| {
					mounts.get(entity).ok().map(|mount| (entity, mount.clone()))
				})
		},
	)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Run a persistent thread to completion: `RunThread` builds + adopts +
	/// mounts the `{MountThreadStore}` store, the mock agent replies, the sync
	/// flushes. One agent turn, no loop, so the run finishes.
	fn run_once(path: &str, system: ActorId, agent: ActorId) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_plugin::<ThreadPlugin>();
		app.world_mut().spawn((
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
		));
		// pump long enough for the async build -> adopt -> mount -> kick -> sync
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
	#[beet_core::test]
	async fn reload_adopts_without_duplicating() {
		let path = "target/tests/beet_thread/run-thread-reload";
		// stable ids so the seed hash matches across runs
		let system = ActorId::from_u128(1);
		let agent = ActorId::from_u128(2);
		BlobThreadStore::new(BlobStore::new(FsStore::new(WsPathBuf::new(path))))
			.store_remove()
			.await
			.ok();

		run_once(path, system, agent);
		stored_threads(path).await.xpect_eq(1);

		// reload: the same seed hash adopts the stored thread, no duplicate
		run_once(path, system, agent);
		stored_threads(path).await.xpect_eq(1);
	}
}
