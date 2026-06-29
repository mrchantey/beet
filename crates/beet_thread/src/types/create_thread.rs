use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Markup verb that runs a thread as a *program*: on load it boots this entity,
/// dispatching its behavior (the thread's `Sequence`, or the loop wrapping it) and
/// holding the process open for the duration of the run.
///
/// Spread it onto a thread's outer root so loading the scene boots the turn loop,
/// with no hand-written kick glue:
///
/// ```rsx
/// <div {(Repeat, CreateThread)}>
///     <div {Thread} {Sequence}> ..actors.. </div>
/// </div>
/// ```
///
/// It rides the one boot path: it `#[require]`s [`BootOnLoad`] (which fires on the
/// entry's `LoadTemplate`) and installs the `Action<Boot, Response>` boot slot that
/// adopts the thread's persistent store, then calls the thread's
/// `Action<(), Outcome>` behavior. A finite loop (`RepeatTimes`,
/// `RepeatWhileFunctionCallOutput`) returns and the boot writes [`AppExit`] to exit;
/// an endless `Repeat` never returns, so the boot call parks and the chat runs until
/// Ctrl+C. Calling the **thread**, never an "agent", is the whole point: the
/// `Sequence` is the behavior and runs its actors in order.
///
/// When the thread carries a [`ThreadStore`] (eg via `{MountThreadStore{path:..}}`)
/// the stored conversation is adopted by seed hash before the first turn, so a
/// reload resumes instead of re-replying; `--new` (carried on the boot args)
/// discards it and starts fresh.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(BootOnLoad)]
#[require(Action<Boot, Response> = Action::new_async_local(run_thread_on_boot))]
pub struct CreateThread;

/// The `Action<Boot, Response>` boot slot [`CreateThread`] installs: adopt the
/// thread's persistent store (honoring `--new` from the boot args), then call the
/// thread's behavior to completion. An empty `Response::ok()` exits successfully;
/// the conversation streams through the actors' own streamers, not this response.
async fn run_thread_on_boot(cx: ActionContext<Boot>) -> Result<Response> {
	let caller = cx.caller.clone();
	let root = caller.id();
	let boot = cx.take();
	// `--new` rides the boot request (built once by `BootOnLoad`), never a global
	// env read mid-load.
	let new = boot.0.get_param("new").is_some();
	adopt_program_store(caller.world().clone(), root, new).await?;
	// Reduce the authored scene into its `ThreadWindow` + behavior scene before
	// running it. The boot drives the behavior directly, ahead of the scheduled
	// `First` reduce, so without this the `Sequence` would receive raw, action-less
	// `<CreateActor>` spans and fail. Idempotent (`Without<ThreadWindow>`), so the
	// store path (already reduced in `adopt_thread`) is unaffected.
	caller
		.world()
		.with(|world: &mut World| ThreadWindow::reduce_now(world))
		.await;
	// run the thread behavior: an endless `Repeat` parks here (holding the process
	// open), a finite loop returns and the boot writes `AppExit`.
	caller.call::<(), Outcome>(()).await?;
	Response::ok().xok()
}

/// Adopt the persistent store of the thread (at or under `root`) declaring a
/// [`MountThreadStore`], by seed hash, then mount it. The store is mounted *after*
/// adoption so the persistence sync never flushes a fresh, un-adopted thread (which
/// would fork a duplicate on every reload). `new` discards the stored conversation
/// first. Ephemeral threads declare no store and skip this.
async fn adopt_program_store(
	world: AsyncWorld,
	root: Entity,
	new: bool,
) -> Result {
	let Some((thread, mount)) = world
		.with(move |world: &mut World| pending_store(world, root))
		.await
	else {
		return Ok(());
	};
	let store = mount.build();
	// `--new` discards the stored conversation and starts fresh
	if new {
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
	Ok(())
}

/// The thread (at or under `root`) declaring a [`MountThreadStore`], with that
/// declaration, if any: the persistence seam [`adopt_program_store`] builds and
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

	/// Run a persistent thread to completion: booting the scene builds + adopts +
	/// mounts the `{MountThreadStore}` store, the mock agent replies, the sync
	/// flushes. One agent turn, no loop, so the run finishes.
	fn run_once(path: &str, system: ActorId, agent: ActorId) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_plugin::<ThreadPlugin>();
		app.world_mut()
			.spawn((
				Thread::default(),
				Sequence::new(),
				CreateThread,
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
			))
			// boot the entry the way `BootOnLoad` does on a real load
			.trigger(|entity| LoadTemplate {
				entity,
				is_error: false,
			});
		// pump long enough for the async boot -> adopt -> mount -> kick -> sync
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
}
