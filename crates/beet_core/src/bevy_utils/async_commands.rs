//! High-level async world access, built on the [`beet_async`] bridge.
//!
//! Async tasks interact with the Bevy [`World`] by *bridging* at a
//! [`BeetAsyncSyncPoint`]: a future enqueues a request, the sync-point driver
//! system publishes `&mut World`, the future runs synchronously and returns its
//! output directly. There is no command channel — every world-accessing method
//! is `async` and must be `.await`ed (an un-awaited bridge future never runs).
//!
//! # Core Types
//!
//! - [`AsyncWorld`] - handle for accessing the world from async contexts
//!   (re-exported from [`beet_async`]; extension methods live on [`AsyncWorldExt`])
//! - [`AsyncEntity`] - handle for operating on a specific entity
//! - [`AsyncCommands`] - system parameter for spawning async tasks from a system
//! - [`AsyncSpawner`] - runtime-agnostic task spawner + in-flight counter
//! - [`AsyncTask`] - a spawned task, cancelled on drop
//!
//! # Task scope
//!
//! A task spawned through an entity (`entity.run_async`, the queued
//! `EntityCommands::queue_async`) is scoped to it: despawning the entity
//! cancels the task. A task spawned through the world (`world.run_async`,
//! [`AsyncCommands::run`]) is unscoped and runs to completion. The `run_task`
//! twins (`queue_task` from commands) return the [`AsyncTask`] instead, for a
//! caller that owns the task itself: a component holding it cancels the task
//! when removed or replaced, and `detach` releases it.
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//!
//! #[derive(Clone, Resource)]
//! struct MyResource(u32);
//!
//! fn my_system(mut commands: AsyncCommands) {
//!     commands.run(async |world| {
//!         world.insert_resource(MyResource(2)).await;
//!         let value = world.resource::<MyResource>().await.0;
//!         assert_eq!(value, 2);
//!     });
//! }
//! ```

use crate::prelude::*;
pub use beet_async::AsyncWorld;
use beet_async::BridgeError;
pub use beet_async::async_world_sync_point;
use bevy::app::MainSchedulePlugin;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::Command;
use bevy::ecs::system::EntityCommand;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::system::RegisteredSystemError;
use bevy::ecs::system::RunSystemError;
use bevy::ecs::system::SystemParam;
use bevy::ecs::template::Template;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::future::Future;
use core::panic::Location;
use core::pin::Pin;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

/// Marker trait for types that are `Send` in multi-threaded environments.
#[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))]
pub trait MaybeSend: Send {}
/// Marker trait for types that are `Send` in multi-threaded environments.
#[cfg(not(all(feature = "bevy_multithreaded", not(target_arch = "wasm32"))))]
pub trait MaybeSend {}
#[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))]
impl<T> MaybeSend for T where T: Send {}
#[cfg(not(all(feature = "bevy_multithreaded", not(target_arch = "wasm32"))))]
impl<T> MaybeSend for T {}

/// Marker trait for types that are `Sync` in multi-threaded environments.
#[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))]
pub trait MaybeSync: Sync {}
/// Marker trait for types that are `Sync` in multi-threaded environments.
#[cfg(not(all(feature = "bevy_multithreaded", not(target_arch = "wasm32"))))]
pub trait MaybeSync {}
#[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))]
impl<T> MaybeSync for T where T: Sync {}
#[cfg(not(all(feature = "bevy_multithreaded", not(target_arch = "wasm32"))))]
impl<T> MaybeSync for T {}

/// The [`SyncPoint`](beet_async) at which beet drives all async world access.
///
/// Registered as an exclusive driver system by [`AsyncPlugin`]. A single sync
/// point is sufficient for beet's needs.
pub struct BeetAsyncSyncPoint;

/// Plugin installing the [`beet_async`] bridge, the [`BeetAsyncSyncPoint`]
/// driver, and the default [`AsyncSpawner`].
///
/// Initializes [`MainSchedulePlugin`] and [`TaskPoolPlugin`] if not present, so
/// it must be added after [`DefaultPlugins`] / [`MinimalPlugins`].
#[derive(Default)]
pub struct AsyncPlugin;

impl Plugin for AsyncPlugin {
	fn build(&self, app: &mut App) {
		// on wasm the bridge drives our tickable executor instead of bevy's
		// JS-event-loop `spawn_local`. Resolved from the world so a sync point
		// only ever polls its own world's tasks (see `beet_async::WorldTicker`).
		#[cfg(all(target_arch = "wasm32", feature = "std"))]
		beet_async::WorldTickerHook::set(|world| {
			world
				.get_resource::<AsyncSpawner>()
				.and_then(AsyncSpawner::ticker)
		});

		app.init_plugin_with(MainSchedulePlugin)
			// drives `tick_global_task_pools_on_main_thread()` in the Last schedule
			.init_plugin::<TaskPoolPlugin>()
			.init_plugin::<beet_async::AsyncPlugin>()
			.init_resource::<AsyncSpawner>()
			.add_systems(
				PreUpdate,
				beet_async::async_world_sync_point::<BeetAsyncSyncPoint>,
			);
	}
}

/// A `'static` future suitable for spawning. `Send` is required only in
/// multi-threaded native builds (matching [`MaybeSend`]).
#[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))]
type SpawnFut = Pin<Box<dyn 'static + Send + Future<Output = ()>>>;
/// A `'static` future suitable for spawning. `Send` is required only in
/// multi-threaded native builds (matching [`MaybeSend`]).
#[cfg(not(all(feature = "bevy_multithreaded", not(target_arch = "wasm32"))))]
type SpawnFut = Pin<Box<dyn 'static + Future<Output = ()>>>;
/// A `'static` future spawned on the local thread, never required to be `Send`.
type SpawnLocalFut = Pin<Box<dyn 'static + Future<Output = ()>>>;

/// Runtime-agnostic task spawner plus an in-flight task counter.
///
/// Spawning is pluggable so a future `tokio` / `embassy` backend can be selected;
/// the default uses [`IoTaskPool`](bevy::tasks::IoTaskPool). The in-flight
/// counter is the idle signal used by [`AsyncRunner`]. Every future is wrapped
/// for cancellation before it reaches the backend, so the [`AsyncTask`] a
/// spawn returns works the same on every runtime.
///
/// ## Every task shares the world thread unless `bevy_multithreaded` is on
///
/// Without that feature (the default, including every deployed `beet-cli`
/// binary) [`spawn`](Self::spawn) is [`spawn_local`](Self::spawn_local): the
/// future joins the *calling* thread's local executor, which for anything
/// spawned from a system or observer is the world-owning thread, ticked by
/// `tick_global_task_pools_on_main_thread`. One server thread then runs the bevy
/// schedule, the async bridge driver, every connection, and every store call.
///
/// So a future that blocks that thread inside `poll` — a `block_on`, blocking
/// file or socket IO, a `std::thread::sleep`, a long synchronous compute — does
/// not merely delay itself, it freezes the world and every in-flight request
/// until it returns, releasing the whole queue at one instant. Awaiting is not
/// optional politeness here, it is what keeps a server responsive. Genuinely
/// blocking work belongs on another thread (`async_ext::on_tokio`, the
/// `blocking` pool), never inline in a spawned task.
#[derive(Resource, Clone)]
pub struct AsyncSpawner(Arc<AsyncSpawnerInner>);

struct AsyncSpawnerInner {
	in_flight: AtomicUsize,
	spawn: Box<dyn Fn(SpawnFut) + Send + Sync>,
	spawn_local: Box<dyn Fn(SpawnLocalFut) + Send + Sync>,
	/// Ticks this spawner's own [`BridgeExecutor`], which it shares with the
	/// spawn functions above. `None` for a spawner built from custom spawn
	/// functions (`tokio`, `embassy`), which drives itself.
	///
	/// Prebuilt rather than made on demand: the sync-point driver resolves it
	/// once per run, and a closure capturing the *spawner* would cycle back
	/// through this `Arc`.
	#[cfg(all(target_arch = "wasm32", feature = "std"))]
	tick: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Default for AsyncSpawner {
	fn default() -> Self {
		// each backend detaches its own handle: cancellation is the `AsyncTask`
		// wrapper `spawn` applies, not the runtime's
		let spawn: Box<dyn Fn(SpawnFut) + Send + Sync>;
		let spawn_local: Box<dyn Fn(SpawnLocalFut) + Send + Sync>;
		cfg_if! {
			// wasm: bevy `spawn_local` uses the JS event loop, which the
			// synchronous bridge driver cannot tick. Use our own tickable
			// executor instead (see `tick_executor`). `std`-gated with
			// the executor itself, so a no_std wasm build falls through to the
			// manual-spawner branch.
			if #[cfg(all(target_arch = "wasm32", feature = "std"))] {
				// one executor per spawner, shared by both spawn functions and
				// the tick below
				let executor = BridgeExecutor::new(alloc::rc::Rc::new(
					async_executor::LocalExecutor::new(),
				));
				let spawn_executor = executor.clone();
				spawn = Box::new(move |fut| {
					spawn_executor.spawn(fut).detach();
				});
				let local_executor = executor.clone();
				spawn_local = Box::new(move |fut| {
					local_executor.spawn(fut).detach();
				});
			} else if #[cfg(all(feature = "std", feature = "bevy_multithreaded"))] {
				spawn = Box::new(|fut| {
					bevy::tasks::IoTaskPool::get().spawn(fut).detach();
				});
				spawn_local = Box::new(|fut| {
					bevy::tasks::IoTaskPool::get().spawn_local(fut).detach();
				});
			} else if #[cfg(feature = "std")] {
				// `SpawnFut` is not `Send` here, so it cannot go through `spawn`
				// (which requires `Send` whenever bevy's `multi_threaded` feature
				// is active); spawn it locally instead.
				spawn = Box::new(|fut| {
					bevy::tasks::IoTaskPool::get().spawn_local(fut).detach();
				});
				spawn_local = Box::new(|fut| {
					bevy::tasks::IoTaskPool::get().spawn_local(fut).detach();
				});
			} else {
				spawn = Box::new(|_| {
					panic!("no default AsyncSpawner on no_std; insert one manually")
				});
				spawn_local = Box::new(|_| {
					panic!("no default AsyncSpawner on no_std; insert one manually")
				});
			}
		}
		Self(Arc::new(AsyncSpawnerInner {
			in_flight: AtomicUsize::new(0),
			spawn,
			spawn_local,
			#[cfg(all(target_arch = "wasm32", feature = "std"))]
			tick: Some(Arc::new(move || tick_executor(&executor)) as Arc<_>),
		}))
	}
}

// `thread_local!` and `async_executor` are both `std`, so a no_std wasm build
// has no bridge executor and supplies its own spawner.
/// One [`AsyncSpawner`]'s tickable executor, and so one world's.
///
/// `SendWrapper` because a `LocalExecutor` is `!Send` while an [`AsyncSpawner`]
/// is both a [`Resource`] and a *detached handle*: [`AsyncCommands`] clones one
/// out of the world and spawns from a task with no world in hand, so the
/// executor has to travel on the handle rather than sit in a `NonSend`
/// resource, which is only reachable through a world. wasm has the one thread,
/// so the wrapper's cross-thread panic is unreachable.
#[cfg(all(target_arch = "wasm32", feature = "std"))]
type BridgeExecutor = send_wrapper::SendWrapper<
	alloc::rc::Rc<async_executor::LocalExecutor<'static>>,
>;

#[cfg(all(target_arch = "wasm32", feature = "std"))]
thread_local! {
	/// The executors with a tick on the stack, keyed by identity.
	static TICKING: core::cell::RefCell<Vec<usize>> = const {
		core::cell::RefCell::new(Vec::new())
	};
	/// How many bridge ticks are on the stack.
	static TICK_DEPTH: core::cell::Cell<usize> = const {
		core::cell::Cell::new(0)
	};
}

/// Poll `executor`'s runnable tasks, so woken futures make progress while the
/// sync-point driver has `&mut World` published.
///
/// The loop is wrapped in [`js_runtime::catch_no_abort`]: wasm has no
/// unwinding, so a task panic is a JS trap that would otherwise propagate into
/// whichever caller ticked (misattributing to an unrelated test's catch scope,
/// or killing the frame). Catching drops the panicking task and defers the rest
/// of the queue to the next tick, mirroring the native `catch_unwind` in
/// `run_async_task_inner`, and buffers the escape for timeout reports. Hosts
/// without a JS catch frame (a production browser app) run the loop directly
/// and keep the trap.
///
/// ## Re-entrancy
/// Ticking is re-entrant by design: a ticked task may drive an app whose sync
/// point ticks again so its bridged requests can poll while that world is
/// published (`beet_async::wake_requests_and_wait`).
///
/// What must never happen is re-entering *the same* executor. Polling a task's
/// siblings is the whole job, but doing it from inside one of their own polls is
/// not: each sibling's runner loop re-enters the executor again, so the stack
/// grows with the number of in-flight tasks rather than with any nesting. A test
/// binary spawns one task per test onto the runner's executor, so that chain
/// reaches any depth cap on load alone. The `TICKING` guard cuts it at the first
/// level, leaving genuine structural nesting (a test driving an app driving a
/// nested runner) to `MAX_TICK_DEPTH`, a backstop against a V8 stack overflow:
/// that surfaces inside arbitrary tests as an opaque panic and, landing in
/// deno's lazy-global machinery, permanently wedges `globalThis.setTimeout` into
/// a `ReferenceError` that kills every later sleep.
#[cfg(all(target_arch = "wasm32", feature = "std"))]
fn tick_executor(executor: &BridgeExecutor) {
	/// nested tick entries permitted before deferring to a later frame; real
	/// structural nesting is ≤5
	const MAX_TICK_DEPTH: usize = 16;
	let key = alloc::rc::Rc::as_ptr(executor) as usize;
	// already ticking this executor further up the stack: its tasks are being
	// polled, and re-entering is the chain described above
	if TICKING.with(|ticking| ticking.borrow().contains(&key)) {
		return;
	}
	let depth = TICK_DEPTH.with(core::cell::Cell::get);
	if depth >= MAX_TICK_DEPTH {
		return;
	}
	TICK_DEPTH.with(|cell| cell.set(depth + 1));
	TICKING.with(|ticking| ticking.borrow_mut().push(key));
	js_runtime::catch_no_abort(|| {
		for _ in 0..100 {
			if !executor.try_tick() {
				break;
			}
		}
		Ok(())
	})
	.map_err(|()| PanicContext::record_swallowed(None))
	.ok();
	TICKING.with(|ticking| {
		ticking.borrow_mut().pop();
	});
	TICK_DEPTH.with(|cell| cell.set(depth));
}

/// Unwrap a bridged closure's output, re-raising a wasm trap the world scope
/// swallowed on its behalf (see the `catch_no_abort` wrap at every `bridge`
/// call site).
///
/// `ScopedStatic::try_with` holds the world-scope mutex *across* the bridged
/// closure, and wasm runs no destructors on a trap, so a panic escaping the
/// closure leaves that mutex locked for the life of the process: every later
/// sync point then panics `cannot recursively acquire mutex` and the async
/// bridge is dead. Catching inside lets the guard release normally, and
/// re-raising here — with the world scope closed — ends the task exactly as
/// the native unwind does.
#[cfg(all(target_arch = "wasm32", feature = "std"))]
fn unwrap_bridged<O>(out: Option<O>) -> O {
	out.unwrap_or_else(|| {
		panic!("bridged world closure panicked, see the panic reported above")
	})
}

impl AsyncSpawner {
	/// Build a spawner from custom spawn functions (eg `tokio` / `embassy`).
	/// Cancellation is applied before a future reaches them (see
	/// [`AsyncTask`]), so they need none of their own.
	pub fn new(
		spawn: impl 'static + Send + Sync + Fn(SpawnFut),
		spawn_local: impl 'static + Send + Sync + Fn(SpawnLocalFut),
	) -> Self {
		Self(Arc::new(AsyncSpawnerInner {
			in_flight: AtomicUsize::new(0),
			spawn: Box::new(spawn),
			spawn_local: Box::new(spawn_local),
			// a custom runtime drives its own tasks
			#[cfg(all(target_arch = "wasm32", feature = "std"))]
			tick: None,
		}))
	}

	/// The number of tasks currently in flight.
	pub fn in_flight(&self) -> usize { self.0.in_flight.load(Ordering::SeqCst) }

	/// The [`WorldTicker`] for this spawner's own executor, which is how the
	/// sync-point driver reaches the tasks of the world it is driving and no
	/// others.
	#[cfg(all(target_arch = "wasm32", feature = "std"))]
	fn ticker(&self) -> Option<beet_async::WorldTicker> {
		self.0
			.tick
			.as_ref()
			.map(|tick| beet_async::WorldTicker::new(tick.clone()))
	}

	/// Poll this spawner's runnable tasks. A no-op for a spawner whose runtime
	/// drives itself.
	#[cfg(all(target_arch = "wasm32", feature = "std"))]
	pub(crate) fn tick(&self) {
		if let Some(tick) = &self.0.tick {
			tick();
		}
	}

	/// Spawns a task, counted in flight until it ends. Dropping the returned
	/// [`AsyncTask`] cancels it; `detach` lets it run unowned.
	pub fn spawn<Fut>(&self, fut: Fut) -> AsyncTask
	where
		Fut: 'static + MaybeSend + Future<Output = ()>,
	{
		let task = AsyncTask::new();
		let counted = self.count(task.wrap(fut));
		(self.0.spawn)(Box::pin(counted));
		task
	}

	/// Spawns a task on the local thread, counted in flight until it ends.
	/// Dropping the returned [`AsyncTask`] cancels it; `detach` lets it run
	/// unowned.
	pub fn spawn_local<Fut>(&self, fut: Fut) -> AsyncTask
	where
		Fut: 'static + Future<Output = ()>,
	{
		let task = AsyncTask::new();
		let counted = self.count(task.wrap(fut));
		(self.0.spawn_local)(Box::pin(counted));
		task
	}

	/// Wraps `fut` so the in-flight counter is held for as long as it lives.
	///
	/// The decrement rides an RAII guard, not the end of the future: a task that
	/// panics or is dropped mid-flight must still release its count, or the
	/// world never reads as idle again and every
	/// [`AsyncRunner::settle_async_tasks`] after it burns the whole frame cap.
	fn count<Fut: Future<Output = ()>>(
		&self,
		fut: Fut,
	) -> impl Future<Output = ()> + use<Fut> {
		struct InFlight(Arc<AsyncSpawnerInner>);
		impl Drop for InFlight {
			fn drop(&mut self) {
				self.0.in_flight.fetch_sub(1, Ordering::SeqCst);
			}
		}
		self.0.in_flight.fetch_add(1, Ordering::SeqCst);
		let guard = InFlight(self.0.clone());
		async move {
			fut.await;
			drop(guard);
		}
	}
}

/// The tasks spawned through an entity, cancelled with it.
///
/// Every entity-scoped spawn ([`EntityWorldMut::run_async`](EntityWorldMutAsyncCommandsExt::run_async),
/// [`AsyncEntity::run_async`], the queued [`EntityCommands`](crate::prelude::EntityCommandsExt::queue_async)
/// variants) pushes here, so despawning the entity drops the handles and
/// cancels the tasks: a server's connections close with it, a reconnect loop
/// ends with its scene. Finished handles are pruned on each push, so a
/// long-lived entity never accumulates one per task it ran. A `run_task` spawn
/// bypasses this: its caller owns the handle.
#[derive(Default, Component)]
#[component(clone_behavior = Ignore)]
struct EntityTasks(Vec<AsyncTask>);

impl EntityTasks {
	/// Own `handle` for the life of `entity`.
	fn push(entity: &mut EntityWorldMut, handle: AsyncTask) {
		match entity.get_mut::<Self>() {
			Some(mut tasks) => {
				tasks.0.retain(|task| !task.is_finished());
				tasks.0.push(handle);
			}
			None => {
				entity.insert(Self(vec![handle]));
			}
		}
	}
}

/// Runs an async task, catching panics (under `std`) and routing any error
/// through the world's error handler.
#[cfg_attr(feature = "nightly", track_caller)]
async fn run_async_task<Func, Fut, Out>(world: AsyncWorld, func: Func)
where
	Func: 'static + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: 'static + IntoResult,
{
	run_async_task_inner(world, None, func).await
}

/// Shared body of [`run_async_task`] and [`run_async_task_entity`]: run `func`,
/// catch panics (under `std`), and route any error through the world's error
/// handler.
///
/// If `guard` is set and that entity has since been despawned, the error is the
/// entity's lifecycle ending (a scene swap, a shutdown), not a fault, so it is
/// logged at `debug` rather than routed to the handler — which panics by default
/// (`Severity::Panic`), and a panic mid-schedule would brick eg robot firmware.
/// An entity-scoped task is cancelled by the despawn, but only at its next
/// pending await (see [`AsyncTask`]): a task despawning its own entity runs
/// out its current poll, and a bridged call it makes there is this path.
#[cfg_attr(feature = "nightly", track_caller)]
async fn run_async_task_inner<Func, Fut, Out>(
	world: AsyncWorld,
	guard: Option<AsyncEntity>,
	func: Func,
) where
	Func: 'static + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: 'static + IntoResult,
{
	let location = Location::caller();
	#[cfg(feature = "std")]
	let result = {
		use futures_lite::future::FutureExt;
		match std::panic::AssertUnwindSafe(func(world.clone()))
			.catch_unwind()
			.await
		{
			Ok(output) => output.into_result(),
			Err(panic) => {
				let msg = display_ext::try_downcast_str(&panic)
					.unwrap_or_else(|| "unknown panic".to_string());
				error!("Async task panicked: {}", msg);
				// the unwind ends here, escaping every test catch scope; buffer
				// it so a hanging test's timeout report can carry the payload
				PanicContext::record_swallowed(Some(msg));
				return;
			}
		}
	};
	// no unwinding to catch under `panic=abort`
	#[cfg(not(feature = "std"))]
	let result = func(world.clone()).await.into_result();

	if let Err(err) = result {
		// suppress the error if this is an entity-scoped task whose entity has
		// gone away — its lifecycle ending, not a fault (see the fn docs).
		let despawned = match &guard {
			Some(entity) => (!entity.is_alive().await).then(|| entity.id()),
			None => None,
		};
		match despawned {
			Some(entity) => {
				debug!("async task for despawned entity {entity} ended: {err}")
			}
			None => {
				world
					.handle_command_error_with_location::<Func>(err, location)
					.await
			}
		}
	}
}

/// Extension methods on the bridged [`AsyncWorld`] handle.
///
/// [`with`](AsyncWorldExt::with) runs on the *exclusive* bridge (raw `&mut World`);
/// [`with_state`](AsyncWorldExt::with_state) runs on the `SystemParam` bridge.
/// All methods are `async`.
#[extend::ext(name=AsyncWorldExt)]
pub impl AsyncWorld {
	/// Runs a function with exclusive `&mut World` access, returning its output.
	///
	/// If the world has been dropped the returned future logs a warning and
	/// never resolves — the spawned task is left for the runtime to clean up.
	#[track_caller]
	fn with<O>(
		&self,
		func: impl 'static + Send + FnOnce(&mut World) -> O,
	) -> impl Future<Output = O> + Send
	where
		O: 'static + Send + Sync,
	{
		let location = Location::caller();
		#[cfg(all(target_arch = "wasm32", feature = "std"))]
		let fut = self.exclusive(BeetAsyncSyncPoint, move |world: &mut World| {
			let mut out = None;
			let _ = js_runtime::catch_no_abort(|| {
				out = Some(func(world));
				Ok(())
			});
			out
		});
		#[cfg(not(all(target_arch = "wasm32", feature = "std")))]
		let fut = self.exclusive(BeetAsyncSyncPoint, func);
		async move {
			match fut.await {
				#[cfg(all(target_arch = "wasm32", feature = "std"))]
				Ok(out) => unwrap_bridged(out),
				#[cfg(not(all(target_arch = "wasm32", feature = "std")))]
				Ok(out) => out,
				Err(BridgeError::WorldDropped) => {
					warn!(
						"AsyncWorld::with: world dropped before bridge completed (at {location}); task will not resume"
					);
					core::future::pending().await
				}
				Err(BridgeError::SystemParamValidation(err)) => {
					// exclusive bridge never validates params; defensive only
					panic!("unexpected SystemParam validation failure: {err}")
				}
			}
		}
	}

	/// Runs a function with access to a [`SystemParam`] via the system-state bridge.
	///
	/// Unlike [`with`](AsyncWorldExt::with), this exposes typed params (queries,
	/// resources, ...) rather than raw `&mut World`.
	///
	/// If the world has been dropped the returned future logs a warning and
	/// never resolves.
	///
	/// # Panics
	///
	/// Panics if the system parameter fails validation, ie a required resource
	/// is missing.
	#[track_caller]
	fn with_state<P: 'static + SystemParam, O>(
		&self,
		func: impl 'static + Send + FnOnce(P::Item<'_, '_>) -> O,
	) -> impl Future<Output = O> + Send
	where
		O: 'static + Send + Sync,
	{
		let location = Location::caller();
		let state = self.system_state::<P>();
		async move {
			#[cfg(all(target_arch = "wasm32", feature = "std"))]
			let bridged = state
				.bridge(BeetAsyncSyncPoint, move |param| {
					let mut out = None;
					let _ = js_runtime::catch_no_abort(|| {
						out = Some(func(param));
						Ok(())
					});
					out
				})
				.await
				.map(unwrap_bridged);
			#[cfg(not(all(target_arch = "wasm32", feature = "std")))]
			let bridged = state.bridge(BeetAsyncSyncPoint, func).await;
			match bridged {
				Ok(out) => out,
				Err(BridgeError::WorldDropped) => {
					warn!(
						"AsyncWorld::with_state: world dropped before bridge completed (at {location}); task will not resume"
					);
					core::future::pending().await
				}
				Err(BridgeError::SystemParamValidation(err)) => {
					panic!("system parameter validation failed: {err}")
				}
			}
		}
	}

	/// Creates an [`AsyncEntity`] handle for the given entity.
	fn entity(&self, entity: Entity) -> AsyncEntity {
		AsyncEntity {
			entity,
			world: self.clone(),
		}
	}

	/// Spawns an entity and returns its [`AsyncEntity`] handle.
	fn spawn<B: Bundle>(
		&self,
		bundle: B,
	) -> impl Future<Output = AsyncEntity> + Send {
		let world = self.clone();
		async move {
			let entity = world
				.with(move |world: &mut World| world.spawn(bundle).id())
				.await;
			world.entity(entity)
		}
	}

	/// Inserts a resource into the world.
	fn insert_resource<R: Resource>(
		&self,
		resource: R,
	) -> impl Future<Output = ()> + Send {
		self.with(move |world: &mut World| {
			world.insert_resource(resource);
		})
	}

	/// Accesses a resource mutably and returns the function's output.
	fn with_resource<R, O>(
		&self,
		func: impl 'static + Send + FnOnce(Mut<R>) -> O,
	) -> impl Future<Output = O> + Send
	where
		R: Resource<Mutability = Mutable>,
		O: 'static + Send + Sync,
	{
		self.with(move |world| func(world.resource_mut::<R>()))
	}

	/// Clones a resource and returns it.
	fn resource<R: Resource + Clone>(&self) -> impl Future<Output = R> + Send {
		self.with(move |world| world.resource::<R>().clone())
	}

	/// Queues a [`Command`] on the world, returning its output.
	fn queue<O>(
		&self,
		command: impl 'static + Send + Command<Out = O>,
	) -> impl Future<Output = O> + Send
	where
		O: 'static + Send + Sync,
	{
		self.with(move |world| command.apply(world))
	}

	/// Triggers an event.
	fn trigger<'a, E: Event<Trigger<'a>: Default>>(
		&self,
		event: E,
	) -> impl Future<Output = ()> + Send {
		self.with(move |world| {
			world.trigger(event);
		})
	}

	/// Writes a message to the world.
	fn write_message<E: Message>(
		&self,
		event: E,
	) -> impl Future<Output = ()> + Send {
		self.with(move |world| {
			world.write_message(event);
		})
	}

	/// Writes a batch of messages to the world.
	fn write_message_batch<E: Message>(
		&self,
		events: impl 'static + Send + IntoIterator<Item = E>,
	) -> impl Future<Output = ()> + Send {
		self.with(move |world| {
			world.write_message_batch(events);
		})
	}

	/// Runs a cached system and returns its output.
	fn run_system_cached<O, M, S>(
		&self,
		system: S,
	) -> impl Future<Output = Result<O, RegisteredSystemError<(), O>>> + Send
	where
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<(), O, M>,
	{
		self.with(move |world| world.run_system_cached(system))
	}

	/// Runs a cached system with input and returns its output.
	fn run_system_cached_with<I, O, M, S>(
		&self,
		system: S,
		input: I::Inner<'static>,
	) -> impl Future<Output = Result<O, RegisteredSystemError<I, O>>> + Send
	where
		I: SystemInput + 'static,
		I::Inner<'static>: Send + Sync,
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<I, O, M>,
	{
		self.with(move |world| world.run_system_cached_with(system, input))
	}

	/// Runs a system once and returns its output.
	fn run_system_once<O, M, S>(
		&self,
		system: S,
	) -> impl Future<Output = Result<O, RunSystemError>> + Send
	where
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<(), O, M>,
	{
		self.with(move |world| world.run_system_once(system))
	}

	/// Runs a system once with input and returns its output.
	fn run_system_once_with<I, O, M, S>(
		&self,
		system: S,
		input: I::Inner<'static>,
	) -> impl Future<Output = Result<O, RunSystemError>> + Send
	where
		I: SystemInput + 'static,
		I::Inner<'static>: Send + Sync,
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<I, O, M>,
	{
		self.with(move |world| world.run_system_once_with(system, input))
	}

	/// Spawns a detached async task.
	fn run_async<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = ()> + Send
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |world| {
			world.run_async(func);
		})
	}

	/// Spawns a detached async task on the local thread.
	fn run_async_local<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = ()> + Send
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |world| {
			world.run_async_local(func);
		})
	}

	/// Spawns an async task, resolving to its [`AsyncTask`] for the caller to
	/// own; [`run_async`](Self::run_async) detaches instead.
	fn run_task<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = AsyncTask> + Send
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |world| world.run_task(func))
	}

	/// Spawns an async task on the local thread, resolving to its
	/// [`AsyncTask`] for the caller to own.
	fn run_task_local<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = AsyncTask> + Send
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |world| world.run_task_local(func))
	}

	/// Registers an observer.
	fn observe<E: Event, B: Bundle, M>(
		&self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> impl Future<Output = ()> + Send {
		self.with(|world| {
			world.add_observer(observer);
		})
	}

	/// Installs a temporary global observer for `E`, resolves on its first fire
	/// with the observer system's return value `O`, then removes the observer.
	///
	/// `observer` is a full observer system, so beyond the [`On<E>`] trigger it
	/// may take arbitrary [`SystemParam`]s and returns the value the future
	/// resolves to, eg `|ev: On<Ready>| ev.entity` to await a
	/// [`Ready`](crate::prelude::Ready) and surface the entity it fired on.
	fn await_event<E, B, M, O>(
		&self,
		observer: impl IntoObserverSystem<E, B, M, O>,
	) -> impl Future<Output = O> + Send
	where
		E: Event<Trigger<'static>: Default>,
		B: Bundle,
		O: 'static + Send + Sync,
	{
		let world = self.clone();
		async move {
			let (send, recv) = OnceValue::oneshot();
			let sender = Arc::new(Mutex::new(Some(send)));
			let slot: Arc<Mutex<Option<Entity>>> = Arc::new(Mutex::new(None));
			let capture_slot = slot.clone();
			// capture the piped observer system's output, signal it once, then
			// despawn the temporary observer.
			let capture =
				move |bevy::ecs::system::In(value): bevy::ecs::system::In<
					O,
				>,
				      mut commands: Commands| {
					if let Some(send) = sender.lock().unwrap().take() {
						send.signal(value);
					}
					if let Some(observer) = capture_slot.lock().unwrap().take()
					{
						commands.entity(observer).despawn();
					}
				};
			let system = IntoObserverSystem::into_system(observer);
			let observer_entity = world
				.with(move |world: &mut World| {
					world.add_observer(system.pipe(capture)).id()
				})
				.await;
			*slot.lock().unwrap() = Some(observer_entity);
			recv.wait().await
		}
	}

	/// Spawns a root and builds `template` into it, awaiting [`Ready`].
	///
	/// Unlike the synchronous [`World::spawn_template`](crate::prelude::WorldTemplateExt),
	/// this awaits the full lifecycle: a template with deferred dependencies
	/// (assets, remote schemas) resolves across later sync points, and the future
	/// completes only once [`Ready`] fires. Fallible: a failed build
	/// returns the root's [`TemplateError`] (its [`CloneError`]).
	fn spawn_template(
		&self,
		template: impl Template<Output = ()> + Send + 'static,
	) -> impl Future<Output = Result<Entity>> + Send {
		build_template_async(self.clone(), None, template)
	}

	/// Raises `err` through the world's configured error handler, attributed to
	/// the call site.
	#[track_caller]
	fn handle_command_error<F>(
		&self,
		err: BevyError,
	) -> impl Future<Output = ()> + Send {
		self.handle_command_error_with_location::<F>(err, Location::caller())
	}

	/// Like [`handle_command_error`](Self::handle_command_error), but attributed
	/// to an explicit `location`, for a raise deferred past its call site.
	fn handle_command_error_with_location<F>(
		&self,
		err: BevyError,
		location: &'static Location<'static>,
	) -> impl Future<Output = ()> + Send {
		self.with(move |world| {
			world.handle_command_error_with_location::<F>(err, location);
		})
	}
}

/// Builds `template` into a root (fresh when `entity` is `None`, else the given
/// entity), awaiting its [`Ready`].
///
/// Installs the [`Ready`] observer *before* building so a synchronous load
/// is never missed, surfacing the root's [`TemplateError`] on failure.
fn build_template_async(
	world: AsyncWorld,
	entity: Option<Entity>,
	template: impl Template<Output = ()> + Send + 'static,
) -> impl Future<Output = Result<Entity>> + Send {
	async move {
		let (send, recv) = OnceValue::oneshot();
		let sender = Arc::new(Mutex::new(Some(send)));
		let root = world
			.with(move |world: &mut World| {
				let root = entity.unwrap_or_else(|| world.spawn_empty().id());
				// observe Ready on the root, signalling whether it failed, then
				// build (which fires it), so a synchronous load is caught.
				world.entity_mut(root).observe(
					move |ev: On<Ready>,
					      errors: Query<(), With<TemplateError>>,
					      mut commands: Commands| {
						let observer = ev.observer();
						if let Some(send) = sender.lock().unwrap().take() {
							send.signal(errors.contains(ev.entity));
						}
						commands.entity(observer).despawn();
					},
				);
				let _ = world.entity_mut(root).insert_template(template);
				root
			})
			.await;
		if recv.wait().await {
			// the root carries the shared `CloneError`; surface it.
			let error = world
				.with(move |world: &mut World| {
					world
						.get::<TemplateError>(root)
						.map(|template_error| template_error.error.clone())
				})
				.await;
			Err(error
				.map(Into::into)
				.unwrap_or_else(|| bevyhow!("template build failed")))
		} else {
			Ok(root)
		}
	}
}

/// A handle for operating on a specific entity from async contexts.
#[derive(Clone)]
pub struct AsyncEntity {
	entity: Entity,
	world: AsyncWorld,
}

impl core::fmt::Debug for AsyncEntity {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("AsyncEntity")
			.field("entity", &self.entity)
			.finish()
	}
}

impl AsyncEntity {
	/// Returns the entity ID.
	pub fn id(&self) -> Entity { self.entity }

	/// Returns a reference to the [`AsyncWorld`].
	pub fn world(&self) -> &AsyncWorld { &self.world }

	/// Returns `true` if the entity still exists in the world.
	pub fn is_alive(&self) -> impl Future<Output = bool> + Send {
		let entity = self.entity;
		self.world
			.with(move |world: &mut World| world.get_entity(entity).is_ok())
	}

	/// Runs a function with access to the entity, returning its output.
	///
	/// Errors if the entity has been despawned.
	pub fn with<O>(
		&self,
		func: impl 'static + Send + FnOnce(EntityWorldMut) -> O,
	) -> impl Future<Output = Result<O>> + Send
	where
		O: 'static + Send + Sync,
	{
		let entity = self.entity;
		self.world.with(move |world: &mut World| -> Result<O> {
			let entity = world
				.get_entity_mut(entity)
				.map_err(|_| bevyhow!("Entity {entity:?} despawned"))?;
			func(entity).xok()
		})
	}

	/// Runs a function with mutable access to the whole [`World`] alongside this
	/// entity's id, returning its output.
	///
	/// Errors if the entity has been despawned. Prefer [`Self::with`] when only
	/// the [`EntityWorldMut`] is needed; reach for this when a handler must touch
	/// the wider world (eg spawn or despawn sibling entities) and still know the
	/// caller's id.
	pub fn with_world<O>(
		&self,
		func: impl 'static + Send + FnOnce(&mut World, Entity) -> O,
	) -> impl Future<Output = Result<O>> + Send
	where
		O: 'static + Send + Sync,
	{
		let entity = self.entity;
		self.world.with(move |world: &mut World| -> Result<O> {
			if world.get_entity(entity).is_err() {
				bevybail!("Entity {entity:?} despawned");
			}
			func(world, entity).xok()
		})
	}

	/// Runs a function with access to the entity id and a [`SystemParam`].
	///
	/// Errors if the entity has been despawned.
	pub fn with_state<P: 'static + SystemParam, O>(
		&self,
		func: impl 'static + Send + FnOnce(Entity, P::Item<'_, '_>) -> O,
	) -> impl Future<Output = Result<O>> + Send
	where
		O: 'static + Send + Sync,
	{
		self.with(|mut entity| entity.with_state::<P, O>(func))
	}

	/// Spawns an async task owned by this entity (cancelled when it despawns),
	/// erroring if the entity has already been despawned.
	pub fn run_async<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = Result<()>> + Send
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |mut entity| {
			entity.run_async(func);
		})
	}

	/// Spawns an async task on the local thread owned by this entity, erroring
	/// if the entity has already been despawned.
	pub fn run_async_local<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = Result<()>> + Send
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |mut entity| {
			entity.run_async_local(func);
		})
	}

	/// Spawns an async task for this entity, resolving to its [`AsyncTask`]
	/// for the caller to own (where [`run_async`](Self::run_async) hands it to
	/// the entity), or erroring if the entity has been despawned.
	pub fn run_task<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = Result<AsyncTask>> + Send
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |mut entity| entity.run_task(func))
	}

	/// Spawns an async task on the local thread for this entity, resolving to
	/// its [`AsyncTask`] for the caller to own, or erroring if the entity has
	/// been despawned.
	pub fn run_task_local<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = Result<AsyncTask>> + Send
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with(move |mut entity| entity.run_task_local(func))
	}

	/// Gets a component and runs a function with it.
	pub fn get<T: Component, O>(
		&self,
		func: impl 'static + Send + FnOnce(&T) -> O,
	) -> impl Future<Output = Result<O>> + Send
	where
		O: 'static + Send + Sync,
	{
		let fut = self.with(move |entity| -> Result<O> {
			if let Some(comp) = entity.get() {
				func(comp).xok()
			} else {
				bevybail!(
					"Component not found: {}",
					core::any::type_name::<T>()
				)
			}
		});
		async move { fut.await.flatten() }
	}

	/// Checks if the entity contains the component.
	pub fn contains<T: Component>(&self) -> impl Future<Output = bool> + Send {
		let fut = self.with(|entity| entity.contains::<T>());
		async move { fut.await.unwrap_or(false) }
	}

	/// Gets a mutable component and runs a function with it.
	pub fn get_mut<T: Component<Mutability = Mutable>, O>(
		&self,
		func: impl 'static + Send + FnOnce(Mut<T>) -> O,
	) -> impl Future<Output = Result<O>> + Send
	where
		O: 'static + Send + Sync,
	{
		let fut = self.with(|mut entity| {
			if let Some(comp) = entity.get_mut() {
				func(comp).xok()
			} else {
				bevybail!(
					"Component not found: {}",
					core::any::type_name::<T>()
				)
			}
		});
		async move { fut.await.flatten() }
	}

	/// Gets a cloned component from an [`AncestorQuery`].
	pub fn get_in_ancestors_cloned<T: Component + Clone>(
		&self,
	) -> impl Future<Output = Result<T>> + Send {
		// AncestorQuery is infallible, with_state will not panic
		let fut = self.with_state::<AncestorQuery<&T>, _>(|entity, query| {
			query.get(entity).cloned().xok()
		});
		async move { fut.await.flatten().flatten() }
	}

	/// Gets a cloned component.
	pub fn get_cloned<T: Component + Clone>(
		&self,
	) -> impl Future<Output = Result<T>> + Send {
		self.get::<T, _>(|comp| comp.clone())
	}

	/// Gets two cloned components.
	pub fn get_cloned2<T1: Component + Clone, T2: Component + Clone>(
		&self,
	) -> impl Future<Output = Result<(T1, T2)>> + Send {
		let fut = self.with(|entity| {
			(
				entity.try_get::<T1>()?.clone(),
				entity.try_get::<T2>()?.clone(),
			)
				.xok()
		});
		async move { fut.await.flatten() }
	}

	/// Takes a component from the entity.
	pub fn take<T: Component>(
		&self,
	) -> impl Future<Output = Result<Option<T>>> + Send {
		self.with(|mut entity| entity.take())
	}

	/// Inserts a bundle into the entity, erroring if the entity has been despawned.
	pub fn insert<B: Bundle>(
		&self,
		bundle: B,
	) -> impl Future<Output = Result<()>> + Send {
		self.with(|mut entity| {
			entity.insert(bundle);
		})
	}

	/// Spawns a child entity and returns its ID.
	pub fn spawn_child<B: Bundle>(
		&self,
		bundle: B,
	) -> impl Future<Output = Entity> + Send {
		let id = self.entity;
		self.world
			.with(move |world| world.spawn((bundle, ChildOf(id))).id())
	}

	/// Queues an [`EntityCommand`] on the entity, returning its output.
	pub fn queue<O>(
		&self,
		command: impl 'static + Send + EntityCommand<Out = O>,
	) -> impl Future<Output = Result<O>> + Send
	where
		O: 'static + Send + Sync,
	{
		self.with(move |entity| command.apply(entity))
	}

	/// Triggers an entity event, erroring if the entity has been despawned.
	pub fn trigger<'t, E: EntityEvent<Trigger<'t>: Default>>(
		&self,
		ev: impl 'static + Send + Sync + FnOnce(Entity) -> E,
	) -> impl Future<Output = Result<()>> + Send {
		self.with(move |mut entity| {
			entity.trigger(ev);
		})
	}

	/// Triggers an entity target event, erroring if the entity has been despawned.
	pub fn trigger_target<M>(
		&self,
		event: impl 'static + Send + IntoEntityTargetEvent<M>,
	) -> impl Future<Output = Result<()>> + Send
	where
		M: 'static,
	{
		self.with(|mut entity| {
			entity.trigger_target(event);
		})
	}

	/// Registers an observer on the entity.
	pub fn observe<E: Event, B: Bundle, M>(
		&self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> impl Future<Output = Result<()>> + Send {
		self.with(|mut entity| {
			entity.observe_any(observer);
		})
	}

	/// Installs a temporary observer for `E` on this entity, resolves on its
	/// first fire with the observer system's return value `O`, then removes the
	/// observer.
	///
	/// `observer` is a full observer system, so beyond the [`On<E>`] trigger it
	/// may take arbitrary [`SystemParam`]s and returns the value the future
	/// resolves to. The route-handler case: `spawn_template` a root, then
	/// `entity.await_event::<Ready, _, _, _>(|ev: On<Ready>, errors: Query<(), With<TemplateError>>| errors.contains(ev.entity))`
	/// to await readiness and branch on the root's error state.
	pub fn await_event<E, B, M, O>(
		&self,
		observer: impl IntoObserverSystem<E, B, M, O>,
	) -> impl Future<Output = Result<O>> + Send
	where
		E: EntityEvent<Trigger<'static>: Default>,
		B: Bundle,
		O: 'static + Send + Sync,
	{
		let world = self.world.clone();
		let target = self.entity;
		async move {
			let (send, recv) = OnceValue::oneshot();
			let sender = Arc::new(Mutex::new(Some(send)));
			// the temporary observer despawns itself on its first fire; its entity
			// is parked here once spawned.
			let slot: Arc<Mutex<Option<Entity>>> = Arc::new(Mutex::new(None));
			let capture_slot = slot.clone();
			// capture the piped observer system's output, signal it once, then
			// despawn the temporary observer.
			let capture =
				move |bevy::ecs::system::In(value): bevy::ecs::system::In<
					O,
				>,
				      mut commands: Commands| {
					if let Some(send) = sender.lock().unwrap().take() {
						send.signal(value);
					}
					if let Some(observer) = capture_slot.lock().unwrap().take()
					{
						commands.entity(observer).despawn();
					}
				};
			let system = IntoObserverSystem::into_system(observer);
			// watch this entity for `E`, piping the user system into the capture.
			let observer_entity = world
				.with(move |world: &mut World| {
					world
						.spawn(
							bevy::ecs::observer::Observer::new(
								system.pipe(capture),
							)
							.with_entity(target),
						)
						.id()
				})
				.await;
			*slot.lock().unwrap() = Some(observer_entity);
			recv.wait().await.xok()
		}
	}

	/// Builds `template` into this entity, awaiting [`Ready`].
	///
	/// The entity-scoped counterpart of
	/// [`AsyncWorld::spawn_template`](AsyncWorldExt::spawn_template):
	/// fallible, awaiting the full lifecycle and surfacing a failed build's
	/// [`TemplateError`].
	pub fn insert_template(
		&self,
		template: impl Template<Output = ()> + Send + 'static,
	) -> impl Future<Output = Result<Entity>> + Send {
		build_template_async(self.world.clone(), Some(self.entity), template)
	}

	/// Despawns the entity.
	pub fn despawn(&self) -> impl Future<Output = Result<()>> + Send {
		self.with(move |entity| {
			entity.despawn();
		})
	}
}

/// System parameter for spawning async tasks from a system, with an
/// [`AsyncWorld`] handle for the spawned task.
#[derive(SystemParam)]
pub struct AsyncCommands<'w, 's> {
	/// Commands for queuing ECS work.
	pub commands: Commands<'w, 's>,
	async_world: Res<'w, AsyncWorld>,
	spawner: Res<'w, AsyncSpawner>,
}

impl AsyncCommands<'_, '_> {
	/// Creates an [`AsyncWorld`] handle for sending commands.
	pub fn world(&self) -> AsyncWorld { self.async_world.clone() }

	/// The [`EntityCommands`] for `entity`, whose
	/// [`queue_async`](crate::prelude::EntityCommandsExt::queue_async) variants
	/// spawn a task scoped to it once the command applies, and whose
	/// [`queue_task`](crate::prelude::EntityCommandsExt::queue_task) variants
	/// hand the task to a component of its own.
	pub fn entity(&mut self, entity: Entity) -> EntityCommands<'_> {
		self.commands.entity(entity)
	}

	/// Spawns a world task, unscoped: it runs to completion whatever despawns.
	pub fn run<Func, Fut, Out>(&self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.run_task(func).detach();
	}

	/// Spawns a world task on the local thread, unscoped.
	pub fn run_local<Func, Fut, Out>(&self, func: Func)
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.run_task_local(func).detach();
	}

	/// Spawns a world task, returning its [`AsyncTask`] for the caller to own;
	/// [`run`](Self::run) detaches instead.
	pub fn run_task<Func, Fut, Out>(&self, func: Func) -> AsyncTask
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.spawner.spawn(run_async_task(self.world(), func))
	}

	/// Spawns a world task on the local thread, returning its [`AsyncTask`]
	/// for the caller to own.
	pub fn run_task_local<Func, Fut, Out>(&self, func: Func) -> AsyncTask
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.spawner.spawn_local(run_async_task(self.world(), func))
	}
}

/// Extension trait adding async command methods to [`World`].
///
/// A world task is unscoped: `run_async` detaches it to run to completion
/// whatever despawns, `run_task` returns its [`AsyncTask`] for the caller to
/// own.
#[extend::ext(name=WorldAsyncCommandsExt)]
pub impl World {
	/// Spawns a detached async task.
	#[track_caller]
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.run_task(func).detach();
		self
	}

	/// Spawns a detached async task on the local thread.
	#[track_caller]
	fn run_async_local<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.run_task_local(func).detach();
		self
	}

	/// Spawns an async task, returning its [`AsyncTask`] for the caller to
	/// own; [`run_async`](Self::run_async) detaches instead.
	#[track_caller]
	fn run_task<Func, Fut, Out>(&mut self, func: Func) -> AsyncTask
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let world = self.resource::<AsyncWorld>().clone();
		self.resource::<AsyncSpawner>()
			.clone()
			.spawn(run_async_task(world, func))
	}

	/// Spawns an async task on the local thread, returning its [`AsyncTask`]
	/// for the caller to own.
	#[track_caller]
	fn run_task_local<Func, Fut, Out>(&mut self, func: Func) -> AsyncTask
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let world = self.resource::<AsyncWorld>().clone();
		self.resource::<AsyncSpawner>()
			.clone()
			.spawn_local(run_async_task(world, func))
	}

	/// Spawns an async task, drives the app to completion, and returns the output.
	#[cfg(feature = "std")]
	#[track_caller]
	fn run_async_then<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = Out>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync,
	{
		let world = self.resource::<AsyncWorld>().clone();
		let (send, recv) = OnceValue::oneshot();
		let spawner = self.resource::<AsyncSpawner>().clone();
		let task = spawner.spawn(async move {
			send.signal(func(world).await);
		});
		async move {
			// held across the drive, so dropping this future cancels the task
			let _task = task;
			AsyncRunner::poll_and_update(
				spawner.clone(),
				|| self.update_local(),
				recv.wait(),
			)
			.await
		}
	}

	/// Spawns a local async task, drives the app to completion, returns the output.
	#[cfg(feature = "std")]
	#[track_caller]
	fn run_async_local_then<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = Out>
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static,
	{
		let world = self.resource::<AsyncWorld>().clone();
		let (send, recv) = OnceValue::oneshot();
		let spawner = self.resource::<AsyncSpawner>().clone();
		let task = spawner.spawn_local(async move {
			send.signal(func(world).await);
		});
		async move {
			// held across the drive, so dropping this future cancels the task
			let _task = task;
			AsyncRunner::poll_and_update(
				spawner.clone(),
				|| self.update_local(),
				recv.wait(),
			)
			.await
		}
	}
}

/// Extension trait adding async command methods to [`EntityWorldMut`].
///
/// The `run_async` variants are entity-scoped: the task's [`AsyncTask`] is
/// owned by the entity and dropped, cancelling it, when the entity despawns.
/// The `run_task` variants return it for the caller to own instead, usually
/// in the component naming the work (`ReloadTail(task)`), so that component's
/// removal or replacement cancels it too. A task that must outlive its entity
/// is a world task, [`World::run_async`](WorldAsyncCommandsExt::run_async).
#[extend::ext(name=EntityWorldMutAsyncCommandsExt)]
pub impl EntityWorldMut<'_> {
	/// Spawns an async task for this entity, cancelled when it despawns.
	///
	/// `Fut` is [`MaybeSend`] (not `Send`), matching the other `run_async`
	/// variants: the future is only sent across threads under
	/// `bevy_multithreaded`, where `MaybeSend` already resolves to `Send`.
	#[track_caller]
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let task = self.run_task(func);
		EntityTasks::push(self, task);
		self
	}

	/// Spawns an async task on the local thread for this entity, cancelled
	/// when it despawns.
	#[track_caller]
	fn run_async_local<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let task = self.run_task_local(func);
		EntityTasks::push(self, task);
		self
	}

	/// Spawns an async task for this entity, returning its [`AsyncTask`] for
	/// the caller to own; [`run_async`](Self::run_async) hands it to the entity
	/// instead. The task still receives this entity's [`AsyncEntity`] and ends
	/// quietly if it finds the entity gone.
	#[track_caller]
	fn run_task<Func, Fut, Out>(&mut self, func: Func) -> AsyncTask
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let id = self.id();
		self.world_scope(move |world| {
			let async_world = world.resource::<AsyncWorld>().clone();
			let entity = async_world.entity(id);
			world
				.resource::<AsyncSpawner>()
				.clone()
				.spawn(run_async_task_entity(entity, func))
		})
	}

	/// Spawns an async task on the local thread for this entity, returning
	/// its [`AsyncTask`] for the caller to own.
	#[track_caller]
	fn run_task_local<Func, Fut, Out>(&mut self, func: Func) -> AsyncTask
	where
		Func: 'static + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let id = self.id();
		self.world_scope(move |world| {
			let async_world = world.resource::<AsyncWorld>().clone();
			let entity = async_world.entity(id);
			world
				.resource::<AsyncSpawner>()
				.clone()
				.spawn_local(run_async_task_entity(entity, func))
		})
	}

	/// Spawns an async task, drives the app to completion, and returns the output.
	#[cfg(feature = "std")]
	#[track_caller]
	fn run_async_then<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = Out>
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync,
	{
		let id = self.id();
		let (send, recv) = OnceValue::oneshot();
		let (async_world, spawner) = self.world_scope(|world| {
			(
				world.resource::<AsyncWorld>().clone(),
				world.resource::<AsyncSpawner>().clone(),
			)
		});
		let entity = async_world.entity(id);
		let task = spawner.spawn(async move {
			send.signal(func(entity).await);
		});
		async move {
			// held across the drive, so dropping this future cancels the task
			let _task = task;
			AsyncRunner::poll_and_update(
				spawner.clone(),
				|| self.world_scope(World::update_local),
				recv.wait(),
			)
			.await
		}
	}

	/// Spawns a local async task, drives the app to completion, returns the output.
	#[cfg(feature = "std")]
	#[track_caller]
	fn run_async_local_then<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = Out>
	where
		Func: 'static + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static,
	{
		let id = self.id();
		let (send, recv) = OnceValue::oneshot();
		let (async_world, spawner) = self.world_scope(|world| {
			(
				world.resource::<AsyncWorld>().clone(),
				world.resource::<AsyncSpawner>().clone(),
			)
		});
		let entity = async_world.entity(id);
		let task = spawner.spawn_local(async move {
			send.signal(func(entity).await);
		});
		async move {
			// held across the drive, so dropping this future cancels the task
			let _task = task;
			AsyncRunner::poll_and_update(
				spawner.clone(),
				|| self.world_scope(World::update_local),
				recv.wait(),
			)
			.await
		}
	}
}

/// Like [`run_async_task`] but threads an [`AsyncEntity`] to the task, and
/// treats that entity being despawned as the task's natural end rather than a
/// fault (see [`run_async_task_inner`]).
async fn run_async_task_entity<Func, Fut, Out>(entity: AsyncEntity, func: Func)
where
	Func: 'static + FnOnce(AsyncEntity) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: 'static + IntoResult,
{
	let world = entity.world().clone();
	let guard = entity.clone();
	run_async_task_inner(world, Some(guard), move |_| func(entity)).await;
}

#[cfg(test)]
mod test {
	use super::EntityTasks;
	use crate::prelude::*;

	fn test_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins.set(TaskPoolPlugin {
			task_pool_options: TaskPoolOptions::with_num_threads(2),
		}));
		app.add_plugins(AsyncPlugin);
		app
	}

	#[derive(Resource, Clone, PartialEq, Debug)]
	struct Count(usize);

	#[crate::test]
	async fn async_task() {
		let mut app = test_app();
		let world = app.world_mut();
		world
			.run_async_then(|world| async move {
				world.insert_resource(Count(0)).await;
				world
					.with_resource::<Count, _>(|mut count| {
						count.0 += 1;
					})
					.await;
			})
			.await;
		world
			.run_async_local_then(|world| async move {
				world.resource::<Count>().await
			})
			.await
			.xpect_eq(Count(1));
	}

	#[crate::test]
	async fn async_queue() {
		let mut app = test_app();
		let world = app.world_mut();
		world.insert_resource(Count(0));

		world
			.run_async_then(|world| async move {
				world
					.with_resource::<Count, _>(|mut count| {
						count.0 += 1;
					})
					.await;
			})
			.await;
		world
			.run_async_then(|world| async move {
				world
					.with_resource::<Count, _>(|mut count| {
						count.0 += 1;
					})
					.await;
			})
			.await;
		world
			.run_async_local_then(|world| async move {
				world.resource::<Count>().await
			})
			.await
			.xpect_eq(Count(2));
	}

	#[crate::test]
	async fn results() {
		let mut app = test_app();
		let world = app.world_mut();
		world.insert_resource(Count(0));

		world
			.run_async_local_then(|world| async move {
				world.resource::<Count>().await
			})
			.await
			.xpect_eq(Count(0));
	}

	#[crate::test]
	async fn run_async_then() {
		let mut app = test_app();
		let result = app.world_mut().run_async_then(|_| async { 42 }).await;
		result.xpect_eq(42);
	}

	#[derive(EntityEvent)]
	struct Ping {
		entity: Entity,
		value: u32,
	}

	#[crate::test]
	async fn await_event_resolves_with_observer_value() {
		let mut app = test_app();
		let entity = app.world_mut().spawn_empty().id();
		// re-fire each update so the awaiting observer catches it whenever it is
		// installed; the observer despawns itself on first fire.
		app.add_systems(Update, move |mut commands: Commands| {
			commands
				.entity(entity)
				.try_trigger(move |entity| Ping { entity, value: 7 });
		});
		let value = app
			.world_mut()
			.run_async_then(move |world| async move {
				world
					.entity(entity)
					.await_event(|ev: On<Ping>| ev.value)
					.await
			})
			.await
			.unwrap();
		value.xpect_eq(7);
	}

	/// Despawning an entity cancels its tasks: a task parked on a gate that
	/// never opens ends with the entity, and its in-flight count is released
	/// through the RAII guard rather than at the end of a future that never
	/// gets there.
	#[crate::test]
	async fn despawn_cancels_entity_tasks() {
		let mut app = test_app();
		let (_gate_send, gate_recv) = OnceValue::<()>::oneshot();
		let entity = app.world_mut().spawn_empty().id();
		app.world_mut()
			.entity_mut(entity)
			.run_async_local(move |_| async move { gate_recv.wait().await });
		// let the task spawn and park on the gate
		AsyncRunner::tick(app.world()).await;
		app.world()
			.resource::<AsyncSpawner>()
			.in_flight()
			.xpect_eq(1);
		app.world_mut().entity_mut(entity).despawn();
		// the runtime drops the cancelled future on its next tick
		AsyncRunner::tick(app.world()).await;
		app.world()
			.resource::<AsyncSpawner>()
			.in_flight()
			.xpect_eq(0);
	}

	/// Finished handles are pruned on each push, so a long-lived entity never
	/// accumulates one per task it ran.
	#[crate::test]
	async fn finished_handles_are_pruned() {
		let mut app = test_app();
		let entity = app.world_mut().spawn_empty().id();
		// gated, so a threaded runtime cannot finish them before they are counted
		let gates = (0..3)
			.map(|_| {
				let (gate_send, gate_recv) = OnceValue::<()>::oneshot();
				app.world_mut().entity_mut(entity).run_async_local(
					move |_| async move { gate_recv.wait().await },
				);
				gate_send
			})
			.collect::<Vec<_>>();
		app.world()
			.get::<EntityTasks>(entity)
			.unwrap()
			.0
			.len()
			.xpect_eq(3);
		// run them to completion, then push once more: only the live handle stays
		for gate in gates {
			gate.signal(());
		}
		AsyncRunner::tick(app.world()).await;
		let (_gate_send, gate_recv) = OnceValue::<()>::oneshot();
		app.world_mut()
			.entity_mut(entity)
			.run_async_local(move |_| async move { gate_recv.wait().await });
		app.world()
			.get::<EntityTasks>(entity)
			.unwrap()
			.0
			.len()
			.xpect_eq(1);
	}

	/// A world task is unscoped: it outlives the despawn of an entity it merely
	/// references, where the entity's own tasks are cancelled.
	#[crate::test]
	async fn world_task_survives_despawn() {
		let mut app = test_app();
		let reached = Store::<bool>::default();
		let (gate_send, gate_recv) = OnceValue::<()>::oneshot();
		let entity = app.world_mut().spawn_empty().id();
		let reached_inner = reached.clone();
		app.world_mut().run_async_local(move |world| async move {
			gate_recv.wait().await;
			world.entity(entity).is_alive().await.xpect_false();
			reached_inner.set(true);
		});
		AsyncRunner::tick(app.world()).await;
		app.world_mut().entity_mut(entity).despawn();
		gate_send.signal(());
		for _ in 0..10 {
			app.update();
			AsyncRunner::tick(app.world()).await;
		}
		reached.get().xpect_true();
	}

	/// The cancellation window: a task despawning its own entity runs out its
	/// current poll, so a bridged call it makes there errors against the gone
	/// entity. That error is the lifecycle ending, suppressed rather than
	/// routed to the (by-default panicking) error handler, which would panic
	/// out of `update()` and fail the test.
	#[crate::test]
	async fn own_despawn_error_is_suppressed() {
		let mut app = test_app();
		let reached = Store::<bool>::default();
		let entity = app.world_mut().spawn_empty().id();
		let reached_inner = reached.clone();
		app.world_mut().entity_mut(entity).run_async_local(
			move |entity| async move {
				entity.despawn().await?;
				reached_inner.set(true);
				// the entity is gone, so this errors
				entity.insert(Name::new("late")).await?;
				Ok(())
			},
		);
		for _ in 0..10 {
			app.update();
			AsyncRunner::tick(app.world()).await;
		}
		reached.get().xpect_true();
	}

	/// A `queue_async_local` task whose entity is despawned *before* the queued
	/// command applies is never spawned (it logs the drop at `debug` instead), so
	/// its body never runs.
	#[crate::test]
	async fn queued_entity_task_skipped_when_despawned_before_run() {
		let mut app = test_app();
		let ran = Store::<bool>::default();
		let entity = app.world_mut().spawn_empty().id();
		let ran_inner = ran.clone();
		// despawn first, then queue: the task's command applies against a gone
		// entity, so it must skip rather than spawn.
		app.world_mut()
			.run_system_once(move |mut commands: Commands| {
				commands.entity(entity).despawn();
				commands.entity(entity).queue_async_local(move |_entity| {
					let ran_inner = ran_inner.clone();
					async move {
						ran_inner.set(true);
					}
				});
			})
			.unwrap();
		for _ in 0..5 {
			app.update();
			AsyncRunner::tick(app.world()).await;
		}
		ran.get().xpect_false();
	}
}
