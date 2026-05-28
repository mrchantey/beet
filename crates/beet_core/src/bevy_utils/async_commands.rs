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
//! - [`AsyncEntityCommands`] - handle for spawning async tasks targeting a
//!   specific entity, built via [`AsyncCommands::entity`]
//! - [`AsyncSpawner`] - runtime-agnostic task spawner + in-flight counter
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
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use core::future::Future;
use core::panic::Location;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::task::Poll;
use core::task::Waker;

/// In wasm or single-threaded environments, wraps this type in a [`SendWrapper`],
/// otherwise is just the type itself.
#[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))]
pub type MaybeSendWrapper<T> = send_wrapper::SendWrapper<T>;
/// In wasm or single-threaded environments, wraps this type in a [`SendWrapper`],
/// otherwise is just the type itself.
#[cfg(not(all(feature = "bevy_multithreaded", not(target_arch = "wasm32"))))]
pub type MaybeSendWrapper<T> = T;

/// Wraps a value in [`MaybeSendWrapper`].
pub fn maybe_send_wrapper<T>(value: T) -> MaybeSendWrapper<T> {
	#[cfg(all(feature = "bevy_multithreaded", not(target_arch = "wasm32")))]
	{
		send_wrapper::SendWrapper::new(value)
	}
	#[cfg(not(all(
		feature = "bevy_multithreaded",
		not(target_arch = "wasm32")
	)))]
	{
		value
	}
}

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
		// JS-event-loop `spawn_local`.
		#[cfg(target_arch = "wasm32")]
		beet_async::set_wasm_tick_hook(tick_bridge_executor);

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
/// counter is the idle signal used by [`AsyncRunner`].
#[derive(Resource, Clone)]
pub struct AsyncSpawner(Arc<AsyncSpawnerInner>);

struct AsyncSpawnerInner {
	in_flight: AtomicUsize,
	spawn: Box<dyn Fn(SpawnFut) + Send + Sync>,
	spawn_local: Box<dyn Fn(SpawnLocalFut) + Send + Sync>,
}

impl Default for AsyncSpawner {
	fn default() -> Self {
		Self(Arc::new(AsyncSpawnerInner {
			in_flight: AtomicUsize::new(0),
			// wasm: bevy `spawn_local` uses the JS event loop, which the
			// synchronous bridge driver cannot tick. Use our own tickable
			// executor instead (see `tick_bridge_executor`).
			#[cfg(target_arch = "wasm32")]
			spawn: Box::new(|fut| {
				BRIDGE_EXECUTOR.with(|exec| exec.spawn(fut).detach());
			}),
			#[cfg(target_arch = "wasm32")]
			spawn_local: Box::new(|fut| {
				BRIDGE_EXECUTOR.with(|exec| exec.spawn(fut).detach());
			}),
			#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
			spawn: Box::new(|fut| {
				bevy::tasks::IoTaskPool::get().spawn(fut).detach();
			}),
			#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
			spawn_local: Box::new(|fut| {
				bevy::tasks::IoTaskPool::get().spawn_local(fut).detach();
			}),
			#[cfg(not(feature = "std"))]
			spawn: Box::new(|_| {
				panic!("no default AsyncSpawner on no_std; insert one manually")
			}),
			#[cfg(not(feature = "std"))]
			spawn_local: Box::new(|_| {
				panic!("no default AsyncSpawner on no_std; insert one manually")
			}),
		}))
	}
}

#[cfg(target_arch = "wasm32")]
thread_local! {
	/// Tickable executor for bridged async tasks on wasm.
	static BRIDGE_EXECUTOR: async_executor::LocalExecutor<'static> =
		async_executor::LocalExecutor::new();
}

/// Ticks the wasm bridge executor so woken futures can poll while the sync-point
/// driver has `&mut World` published. Registered with [`beet_async`] as its
/// wasm tick hook.
#[cfg(target_arch = "wasm32")]
pub(crate) fn tick_bridge_executor() {
	BRIDGE_EXECUTOR.with(|exec| {
		for _ in 0..100 {
			if !exec.try_tick() {
				break;
			}
		}
	});
}

impl AsyncSpawner {
	/// Build a spawner from custom spawn functions (eg `tokio` / `embassy`).
	pub fn new(
		spawn: impl 'static + Send + Sync + Fn(SpawnFut),
		spawn_local: impl 'static + Send + Sync + Fn(SpawnLocalFut),
	) -> Self {
		Self(Arc::new(AsyncSpawnerInner {
			in_flight: AtomicUsize::new(0),
			spawn: Box::new(spawn),
			spawn_local: Box::new(spawn_local),
		}))
	}

	/// The number of tasks currently in flight.
	pub fn in_flight(&self) -> usize { self.0.in_flight.load(Ordering::SeqCst) }

	/// Spawns a task, incrementing the in-flight counter until it completes.
	pub fn spawn<Fut>(&self, fut: Fut)
	where
		Fut: 'static + MaybeSend + Future<Output = ()>,
	{
		self.0.in_flight.fetch_add(1, Ordering::SeqCst);
		let this = self.clone();
		(self.0.spawn)(Box::pin(async move {
			fut.await;
			this.0.in_flight.fetch_sub(1, Ordering::SeqCst);
		}));
	}

	/// Spawns a task on the local thread, incrementing the in-flight counter
	/// until it completes.
	pub fn spawn_local<Fut>(&self, fut: Fut)
	where
		Fut: 'static + Future<Output = ()>,
	{
		self.0.in_flight.fetch_add(1, Ordering::SeqCst);
		let this = self.clone();
		(self.0.spawn_local)(Box::pin(async move {
			fut.await;
			this.0.in_flight.fetch_sub(1, Ordering::SeqCst);
		}));
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
	#[cfg(feature = "std")]
	{
		use futures_lite::future::FutureExt;
		let result = std::panic::AssertUnwindSafe(func(world.clone()))
			.catch_unwind()
			.await;
		match result {
			Ok(output) => {
				if let Err(err) = output.into_result() {
					let location = Location::caller();
					world.handle_command_error::<Func>(err, location).await;
				}
			}
			Err(panic) => {
				let msg = display_ext::try_downcast_str(&panic)
					.unwrap_or_else(|| "unknown panic".to_string());
				cross_log!("Async task panicked: {}", msg);
			}
		}
	}
	#[cfg(not(feature = "std"))]
	{
		// no unwinding to catch under `panic=abort`
		if let Err(err) = func(world.clone()).await.into_result() {
			let location = Location::caller();
			world.handle_command_error::<Func>(err, location).await;
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
		let fut = self.exclusive(BeetAsyncSyncPoint, func);
		async move {
			match fut.await {
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
			match state.bridge(BeetAsyncSyncPoint, func).await {
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

	/// Spawns an async task.
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

	/// Spawns an async task on the local thread.
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

	/// Registers an observer.
	fn observe<E: Event, B: Bundle, M>(
		&self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> impl Future<Output = ()> + Send {
		self.with(|world| {
			world.add_observer(observer);
		})
	}

	/// Awaits a single event of type `E`, then despawns the observer.
	fn await_event<E: Event, B: Bundle>(
		&self,
	) -> impl Future<Output = ()> + Send {
		let world = self.clone();
		async move {
			let signal = OnceSignal::new();
			let observer_signal = signal.clone();
			world
				.observe(move |ev: On<E, B>, mut commands: Commands| {
					observer_signal.signal();
					commands.entity(ev.observer()).despawn();
				})
				.await;
			signal.wait().await;
		}
	}

	/// Handles an error using the world's default error handler.
	fn handle_command_error<F>(
		&self,
		err: BevyError,
		location: &'static Location<'static>,
	) -> impl Future<Output = ()> + Send {
		self.with(move |world| {
			world.handle_command_error::<F>(err, location);
		})
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
		self.with(|mut entity| entity.with_state(func))
	}

	/// Spawns an async task for this entity, erroring if the entity has been despawned.
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

	/// Spawns an async task on the local thread for this entity, erroring if the entity has been despawned.
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

	/// Awaits a single event of type `E` for this entity, then despawns the observer.
	pub fn await_event<E: Event, B: Bundle>(
		&self,
	) -> impl Future<Output = Result<()>> + Send {
		let this = self.clone();
		async move {
			let signal = OnceSignal::new();
			let observer_signal = signal.clone();
			this.observe(move |ev: On<E, B>, mut commands: Commands| {
				observer_signal.signal();
				commands.entity(ev.observer()).despawn();
			})
			.await?;
			signal.wait().await;
			Ok(())
		}
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

	/// Creates an [`AsyncEntityCommands`] handle for spawning async tasks
	/// targeting a specific entity.
	pub fn entity(&self, entity: Entity) -> AsyncEntityCommands {
		AsyncEntityCommands {
			entity,
			world: self.world(),
			spawner: (*self.spawner).clone(),
		}
	}

	/// Spawns an async task that can access the world.
	pub fn run<Func, Fut, Out>(&self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.spawner.spawn(run_async_task(self.world(), func));
	}

	/// Spawns an async task on the local thread.
	pub fn run_local<Func, Fut, Out>(&self, func: Func)
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.spawner.spawn_local(run_async_task(self.world(), func));
	}
}

/// Handle for spawning async tasks targeting a specific entity.
///
/// Built via [`AsyncCommands::entity`]; spawned tasks receive an
/// [`AsyncEntity`] for the target entity.
#[derive(Clone)]
pub struct AsyncEntityCommands {
	entity: Entity,
	world: AsyncWorld,
	spawner: AsyncSpawner,
}

impl AsyncEntityCommands {
	/// Returns the target entity ID.
	pub fn id(&self) -> Entity { self.entity }

	/// Returns an [`AsyncEntity`] handle for the target entity.
	pub fn async_entity(&self) -> AsyncEntity { self.world.entity(self.entity) }

	/// Spawns an async task with an [`AsyncEntity`] handle for the target entity.
	pub fn run<Func, Fut, Out>(&self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.spawner
			.spawn(run_async_task_entity(self.async_entity(), func));
	}

	/// Spawns an async task on the local thread with an [`AsyncEntity`] handle
	/// for the target entity.
	pub fn run_local<Func, Fut, Out>(&self, func: Func)
	where
		Func: 'static + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.spawner
			.spawn_local(run_async_task_entity(self.async_entity(), func));
	}
}

/// Extension trait adding async command methods to [`World`].
#[extend::ext(name=WorldAsyncCommandsExt)]
pub impl World {
	/// Spawns an async task.
	#[track_caller]
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let world = self.resource::<AsyncWorld>().clone();
		self.resource::<AsyncSpawner>()
			.clone()
			.spawn(run_async_task(world, func));
		self
	}

	/// Spawns an async task on the local thread.
	#[track_caller]
	fn run_async_local<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let world = self.resource::<AsyncWorld>().clone();
		self.resource::<AsyncSpawner>()
			.clone()
			.spawn_local(run_async_task(world, func));
		self
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
		let (send, recv) = oneshot();
		self.resource::<AsyncSpawner>().clone().spawn(async move {
			send.signal(func(world).await);
		});
		AsyncRunner::poll_and_update(|| self.update_local(), recv.wait())
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
		let (send, recv) = oneshot();
		self.resource::<AsyncSpawner>()
			.clone()
			.spawn_local(async move {
				send.signal(func(world).await);
			});
		AsyncRunner::poll_and_update(|| self.update_local(), recv.wait())
	}
}

/// Extension trait adding async command methods to [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutAsyncCommandsExt)]
pub impl EntityWorldMut<'_> {
	/// Spawns an async task for this entity.
	#[track_caller]
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let id = self.id();
		self.world_scope(move |world| {
			let async_world = world.resource::<AsyncWorld>().clone();
			let entity = async_world.entity(id);
			world
				.resource::<AsyncSpawner>()
				.clone()
				.spawn(run_async_task_entity(entity, func));
		});
		self
	}

	/// Spawns an async task on the local thread for this entity.
	#[track_caller]
	fn run_async_local<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
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
				.spawn_local(run_async_task_entity(entity, func));
		});
		self
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
		let (send, recv) = oneshot();
		let (async_world, spawner) = self.world_scope(|world| {
			(
				world.resource::<AsyncWorld>().clone(),
				world.resource::<AsyncSpawner>().clone(),
			)
		});
		let entity = async_world.entity(id);
		spawner.spawn(async move {
			send.signal(func(entity).await);
		});
		AsyncRunner::poll_and_update(
			|| self.world_scope(World::update_local),
			recv.wait(),
		)
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
		let (send, recv) = oneshot();
		let (async_world, spawner) = self.world_scope(|world| {
			(
				world.resource::<AsyncWorld>().clone(),
				world.resource::<AsyncSpawner>().clone(),
			)
		});
		let entity = async_world.entity(id);
		spawner.spawn_local(async move {
			send.signal(func(entity).await);
		});
		AsyncRunner::poll_and_update(
			|| self.world_scope(World::update_local),
			recv.wait(),
		)
	}
}

/// Like [`run_async_task`] but threads an [`AsyncEntity`] to the task.
async fn run_async_task_entity<Func, Fut, Out>(entity: AsyncEntity, func: Func)
where
	Func: 'static + FnOnce(AsyncEntity) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: 'static + IntoResult,
{
	let world = entity.world().clone();
	run_async_task(world, move |_| func(entity)).await;
}

/// A one-shot value channel: a single value is published once and the awaiting
/// task is woken. Used for the `run_async_*_then` result hand-off (std-only,
/// since those drive the app via [`AsyncRunner`]).
#[cfg(feature = "std")]
fn oneshot<T>() -> (OnceValue<T>, OnceValueRx<T>) {
	let inner = Arc::new(OnceValueInner {
		value: Mutex::new(None),
		waker: Mutex::new(None),
		set: AtomicBool::new(false),
	});
	(OnceValue(inner.clone()), OnceValueRx(inner))
}

#[cfg(feature = "std")]
struct OnceValueInner<T> {
	value: Mutex<Option<T>>,
	waker: Mutex<Option<Waker>>,
	set: AtomicBool,
}

#[cfg(feature = "std")]
struct OnceValue<T>(Arc<OnceValueInner<T>>);
#[cfg(feature = "std")]
struct OnceValueRx<T>(Arc<OnceValueInner<T>>);

#[cfg(feature = "std")]
impl<T> OnceValue<T> {
	fn signal(self, value: T) {
		*self.0.value.lock().unwrap() = Some(value);
		self.0.set.store(true, Ordering::SeqCst);
		if let Some(waker) = self.0.waker.lock().unwrap().take() {
			waker.wake();
		}
	}
}

#[cfg(feature = "std")]
impl<T> OnceValueRx<T> {
	fn wait(self) -> impl Future<Output = T> {
		core::future::poll_fn(move |cx| {
			if self.0.set.load(Ordering::SeqCst) {
				Poll::Ready(self.0.value.lock().unwrap().take().unwrap())
			} else {
				*self.0.waker.lock().unwrap() = Some(cx.waker().clone());
				Poll::Pending
			}
		})
	}
}

/// A clonable one-shot completion flag used by `await_event`.
#[derive(Clone)]
struct OnceSignal(Arc<OnceSignalInner>);

struct OnceSignalInner {
	fired: AtomicBool,
	waker: Mutex<Option<Waker>>,
}

impl OnceSignal {
	fn new() -> Self {
		Self(Arc::new(OnceSignalInner {
			fired: AtomicBool::new(false),
			waker: Mutex::new(None),
		}))
	}

	fn signal(&self) {
		self.0.fired.store(true, Ordering::SeqCst);
		if let Some(waker) = self.0.waker.lock().unwrap().take() {
			waker.wake();
		}
	}

	fn wait(&self) -> impl Future<Output = ()> + '_ {
		core::future::poll_fn(move |cx| {
			if self.0.fired.load(Ordering::SeqCst) {
				Poll::Ready(())
			} else {
				*self.0.waker.lock().unwrap() = Some(cx.waker().clone());
				Poll::Pending
			}
		})
	}
}

#[cfg(test)]
mod test {
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

	#[beet_core::test]
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

	#[beet_core::test]
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

	#[beet_core::test]
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

	#[beet_core::test]
	async fn run_async_then() {
		let mut app = test_app();
		let result = app.world_mut().run_async_then(|_| async { 42 }).await;
		result.xpect_eq(42);
	}
}
