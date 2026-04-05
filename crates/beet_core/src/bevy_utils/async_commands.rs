//! Async command execution for Bevy.
//!
//! This module provides infrastructure for running async tasks that can interact
//! with the Bevy [`World`] through a channel-based command queue system.
//!
//! # Core Types
//!
//! - [`AsyncCommands`] - System parameter for spawning async tasks
//! - [`AsyncWorld`] - Handle for sending commands from async contexts
//! - [`AsyncEntity`] - Handle for operating on a specific entity from async contexts
//! - [`AsyncChannel`] - Resource managing the command queue channel
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
//!         world.insert_resource(MyResource(2));
//!         let value = world.resource::<MyResource>().await.0;
//!         assert_eq!(value, 2);
//!     });
//! }
//! ```

use crate::prelude::*;
use async_channel;
use async_channel::Receiver;
use async_channel::Sender;
use bevy::app::MainSchedulePlugin;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::system::RegisteredSystemError;
use bevy::ecs::system::RunSystemError;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::CommandQueue;
use bevy::ecs::world::WorldId;
use bevy::tasks::IoTaskPool;
use std::future::Future;
use std::panic::Location;

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


/// Plugin that polls background async work and applies produced [`CommandQueue`]s
/// to the main Bevy world.
///
/// This plugin initializes [`TaskPoolPlugin`] and [`MainSchedulePlugin`] if not present,
/// so it must be added after [`DefaultPlugins`] / [`MinimalPlugins`].
#[derive(Default)]
pub struct AsyncPlugin;

impl Plugin for AsyncPlugin {
	fn build(&self, app: &mut App) {
		AsyncWorld::register(app.world_mut());
		app.init_plugin_with(MainSchedulePlugin)
			// this will add the system to tick_global_task_pools_on_main_thread() in the Last schedule
			.init_plugin::<TaskPoolPlugin>()
			.add_systems(PreUpdate, append_async_queues);
	}
}

/// Appends all [`AsyncChannel::rx`] command queues directly to the world.
fn append_async_queues(world: &mut World) -> Result {
	// Clone the receiver to avoid borrow conflict
	let rx = world.get_resource::<AsyncChannel>().map(|c| c.rx.clone());
	let Some(rx) = rx else {
		return Ok(());
	};

	while let Ok(mut queue) = rx.try_recv() {
		queue.apply(world);
	}
	Ok(())
}

#[cfg_attr(feature = "nightly", track_caller)]
async fn run_async_task<Func, Fut, Out>(world: AsyncWorld, func: Func) -> Result
where
	Func: 'static + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: 'static + Send + Sync + IntoResult,
{
	use futures_lite::future::FutureExt;

	let world3 = world.clone();
	// 1. increment task count
	world3
		.with_resource_then::<AsyncChannel, _>(|mut channel| {
			channel.increment_tasks();
		})
		.await;
	// 2. run the function, catching panics
	let result = std::panic::AssertUnwindSafe(func(world))
		.catch_unwind()
		.await;
	// 3. decrement task count
	world3
		.with_resource_then::<AsyncChannel, _>(|mut channel| {
			channel.decrement_tasks();
		})
		.await;

	// 4. handle result
	match result {
		Ok(output) => {
			if let Err(err) = output.into_result() {
				let location = std::panic::Location::caller();
				world.handle_command_error::<Func>(err, location).await;
			}
		}
		Err(panic) => {
			let msg = display_ext::try_downcast_str(&panic)
				.unwrap_or_else(|| "unknown panic".to_string());
			cross_log!("Async task panicked: {}", msg);
		}
	}

	Ok(())
}

#[track_caller]
fn spawn_async_task<Func, Fut, Out>(world: AsyncWorld, func: Func)
where
	Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + MaybeSend + Future<Output = Out>,
	Out: 'static + Send + Sync + IntoResult,
{
	IoTaskPool::get()
		.spawn(run_async_task(world, func))
		// TODO this means we cant clean tasks up, instead they should be stored
		// in world so when world is dropped so are tasks
		.detach();
}

#[track_caller]
fn spawn_async_task_local<Func, Fut, Out>(world: AsyncWorld, func: Func)
where
	Func: 'static + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: 'static + Send + Sync + IntoResult,
{
	IoTaskPool::get()
		.spawn_local(run_async_task(world, func))
		// TODO this means we cant clean tasks up, instead they should be stored
		// in world so when world is dropped so are tasks
		.detach();
}

/// Spawns the async task, flushes all async tasks and returns the output.
async fn spawn_async_task_then<Func, Fut, Out>(
	world: AsyncWorld,
	update: impl FnMut(),
	func: Func,
) -> Out
where
	Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + MaybeSend + Future<Output = Out>,
	Out: 'static + Send + Sync,
{
	let (send, recv) = async_channel::bounded(1);
	spawn_async_task(world, async move |world| {
		let out = func(world).await;
		// allowed to drop recv
		send.try_send(out).ok();
	});
	AsyncRunner::poll_and_update(update, recv)
		.await
		.expect("spawn_async_task_then channel closed: task likely panicked")
}

/// Spawns the async local task, flushes all async tasks and returns the output.
async fn spawn_async_task_local_then<Func, Fut, Out>(
	world: AsyncWorld,
	update: impl FnMut(),
	func: Func,
) -> Out
where
	Func: 'static + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: 'static,
{
	let (send, recv) = async_channel::bounded(1);
	spawn_async_task_local(world, async move |world| {
		let out = func(world).await;
		// allowed to drop recv
		send.try_send(out).ok();
	});
	AsyncRunner::poll_and_update(update, recv).await.expect(
		"spawn_async_task_local_then channel closed: task likely panicked",
	)
}


/// System parameter for running async functions that can interact with the [`World`].
///
/// Provides an [`AsyncWorld`] handle to the async function, which can be used to
/// send commands back to the main world.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
///
/// #[derive(Clone, Resource)]
/// struct MyResource(u32);
///
/// fn my_system(mut commands: AsyncCommands) {
///     commands.run(async |world| {
///         world.insert_resource(MyResource(2));
///         let value = world.resource::<MyResource>().await.0;
///         assert_eq!(value, 2);
///     });
/// }
/// ```
#[derive(SystemParam)]
pub struct AsyncCommands<'w, 's> {
	/// The commands used for spawning entities.
	pub commands: Commands<'w, 's>,
	/// The channel used to create an [`AsyncWorld`] passed to the async function.
	pub world_id: WorldId,
}


impl<'w, 's> AsyncCommands<'w, 's> {
	/// Reborrows the commands with a shorter lifetime.
	pub fn reborrow(&mut self) -> AsyncCommands<'w, '_> {
		AsyncCommands {
			commands: self.commands.reborrow(),
			world_id: self.world_id,
		}
	}

	/// Creates an [`AsyncWorld`] handle for sending commands.
	pub fn world(&self) -> AsyncWorld { AsyncWorld::new(self.world_id) }

	/// Spawns an async task that can send commands to the world.
	pub fn run<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		spawn_async_task(self.world(), func);
	}

	/// Spawns an async task on the local thread.
	pub fn run_local<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		spawn_async_task_local(self.world(), func);
	}
}

/// Resource containing the channel used by async functions to send [`CommandQueue`]s.
#[derive(Clone, Resource)]
pub struct AsyncChannel {
	/// The number of tasks currently in flight.
	task_count: usize,
	/// The receiver for the async channel.
	rx: Receiver<CommandQueue>,
}

impl AsyncChannel {
	fn new(recv: Receiver<CommandQueue>) -> Self {
		Self {
			rx: recv,
			task_count: 0,
		}
	}

	/// Returns the number of tasks currently in flight.
	pub fn task_count(&self) -> usize { self.task_count }

	fn increment_tasks(&mut self) -> &mut Self {
		self.task_count += 1;
		self
	}

	fn decrement_tasks(&mut self) -> &mut Self {
		self.task_count = self.task_count.saturating_sub(1);
		self
	}
}

/// A portable handle for sending [`CommandQueue`]s to the world from async contexts.
///
/// Any async function that accepts a single [`AsyncWorld`] argument is an async system.
/// This type is [`Copy`] as it only stores a [`WorldId`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AsyncWorld {
	world_id: WorldId,
}

impl From<WorldId> for AsyncWorld {
	fn from(world_id: WorldId) -> Self { Self::new(world_id) }
}

static WORLD_SENDERS: std::sync::LazyLock<
	std::sync::Mutex<HashMap<WorldId, Sender<CommandQueue>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));


impl AsyncWorld {
	/// Creates a new [`AsyncWorld`] from a command queue sender.
	pub fn new(world_id: WorldId) -> Self { Self { world_id } }

	fn register(world: &mut World) {
		let mut senders = WORLD_SENDERS.lock().unwrap();
		if !senders.contains_key(&world.id()) {
			let (send, recv) = async_channel::unbounded();
			senders.insert(world.id(), send);
			world.insert_resource(AsyncChannel::new(recv));
		}
	}

	fn send(&self, queue: CommandQueue) {
		let senders = WORLD_SENDERS.lock().unwrap();
		let sender = senders.get(&self.world_id).expect(
			"AsyncWorld sender not found for world, please add the AsyncPlugin",
		);

		if let Err(err) = sender.try_send(queue) {
			// we dont unwrap here, its common for this to happen
			warn!("Failed to send command queue: {}", err);
		}
	}

	/// Queues a command to be executed on the world.
	pub fn with(&self, func: impl Command + FnOnce(&mut World)) {
		let mut queue = CommandQueue::default();
		queue.push(func);
		self.send(queue);
	}

	/// Queues a command and returns a future that resolves when the command completes.
	pub fn with_then<O>(
		&self,
		func: impl 'static + Send + FnOnce(&mut World) -> O,
	) -> impl Future<Output = O>
	where
		O: 'static + Send + Sync,
	{
		let (out_tx, out_rx) = async_channel::bounded(1);
		let mut queue = CommandQueue::default();
		queue.push(move |world: &mut World| {
			let out = func(world);
			out_tx
				.try_send(out)
				// allow dropped, they didnt want the output
				.ok();
		});
		self.send(queue);
		async move {
			match out_rx.recv().await {
				Ok(out) => out,
				Err(_) => {
					// Channel closed - world was dropped during teardown.
					// Abort by pending forever; the runtime will clean us up.
					std::future::pending().await
				}
			}
		}
	}

	/// Creates an [`AsyncEntity`] handle for the given entity.
	pub fn entity(&self, entity: Entity) -> AsyncEntity {
		AsyncEntity {
			entity,
			world: self.clone(),
		}
	}

	/// Spawns an entity with the given bundle.
	pub fn spawn<B: Bundle>(&self, bundle: B) {
		self.with(move |world: &mut World| {
			world.spawn(bundle);
		});
	}

	/// Spawns an entity and returns its [`AsyncEntity`] handle.
	pub fn spawn_then<B: Bundle>(
		&self,
		bundle: B,
	) -> impl Future<Output = AsyncEntity> {
		async move {
			let entity = self
				.with_then(move |world: &mut World| world.spawn(bundle).id())
				.await;
			self.entity(entity)
		}
	}

	/// Inserts a resource into the world.
	pub fn insert_resource<R: Resource>(&self, resource: R) {
		self.with(move |world: &mut World| {
			world.insert_resource(resource);
		});
	}

	/// Inserts a resource and waits for completion.
	pub async fn insert_resource_then<R: Resource>(&self, resource: R) {
		self.with_then(move |world: &mut World| {
			world.insert_resource(resource);
		})
		.await;
	}

	/// Accesses a resource mutably.
	pub fn with_resource<R: Resource>(
		&self,
		func: impl FnOnce(Mut<R>) + Send + 'static,
	) {
		self.with(move |world| {
			func(world.resource_mut::<R>());
		});
	}

	/// Accesses a resource mutably and returns a future with the result.
	pub fn with_resource_then<R, O>(
		&self,
		func: impl 'static + Send + FnOnce(Mut<R>) -> O,
	) -> impl Future<Output = O>
	where
		R: Resource,
		O: 'static + Send + Sync,
	{
		self.with_then(move |world| func(world.resource_mut::<R>()))
	}
	/// Runs a function with access to a system parameter state.
	pub fn with_state<T: 'static + SystemParam, O>(
		&self,
		func: impl 'static + Send + FnOnce(T::Item<'_, '_>) -> O,
	) -> impl Future<Output = O>
	where
		O: 'static + Send + Sync,
	{
		self.with_then(|world| world.with_state(func))
	}

	/// Clones a resource and returns it.
	pub fn resource<R: Resource + Clone>(&self) -> impl Future<Output = R>
	where
		R: Resource,
	{
		self.with_then(move |world| world.resource::<R>().clone())
	}

	/// Triggers an event.
	pub fn trigger<'a, E: Event<Trigger<'a>: Default>>(&self, event: E) {
		self.with(move |world| {
			world.trigger(event);
		});
	}

	/// Writes a message to the world.
	pub fn write_message<E: Message>(&self, event: E) {
		self.with(move |world| {
			world.write_message(event);
		});
	}

	/// Writes a batch of messages to the world.
	pub fn write_message_batch<E: Message>(
		&self,
		event: impl 'static + Send + IntoIterator<Item = E>,
	) {
		self.with(move |world| {
			world.write_message_batch(event);
		});
	}

	/// Runs a cached system and returns its output.
	pub async fn run_system_cached<O, M, S>(
		&self,
		system: S,
	) -> Result<O, RegisteredSystemError<(), O>>
	where
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<(), O, M>,
	{
		self.run_system_cached_with(system, ()).await
	}

	/// Runs a cached system with input and returns its output.
	pub async fn run_system_cached_with<I, O, M, S>(
		&self,
		system: S,
		input: I::Inner<'_>,
	) -> Result<O, RegisteredSystemError<I, O>>
	where
		I: SystemInput + 'static,
		for<'a> I::Inner<'a>: 'static + Send + Sync,
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<I, O, M>,
	{
		self.with_then(move |world| world.run_system_cached_with(system, input))
			.await
	}

	/// Runs a system once and returns its output.
	pub async fn run_system_once<O, M, S>(
		&self,
		system: S,
	) -> Result<O, RunSystemError>
	where
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<(), O, M>,
	{
		self.run_system_once_with(system, ()).await
	}

	/// Runs a system once with input and returns its output.
	pub async fn run_system_once_with<I, O, M, S>(
		&self,
		system: S,
		input: I::Inner<'_>,
	) -> Result<O, RunSystemError>
	where
		I: SystemInput + 'static,
		for<'a> I::Inner<'a>: 'static + Send + Sync,
		O: 'static + Send + Sync,
		S: 'static + Send + IntoSystem<I, O, M>,
	{
		self.with_then(move |world| world.run_system_once_with(system, input))
			.await
	}

	/// Spawns an async task.
	pub fn run_async<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = ()>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with_then(move |world| {
			world.run_async(func);
		})
	}

	/// Spawns an async task on the local thread.
	pub fn run_async_local<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = ()>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with_then(move |world| {
			world.run_async_local(func);
		})
	}

	/// Registers an observer.
	pub async fn observe<E: Event, B: Bundle, M>(
		&self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> &Self {
		self.with_then(|world| {
			world.add_observer(observer);
		})
		.await;
		self
	}

	/// Awaits a single event of type `E`, then despawns the observer.
	pub async fn await_event<E: Event, B: Bundle>(&self) -> &Self {
		let (send, recv) = async_channel::bounded(1);
		self.observe(move |ev: On<E, B>, mut commands: Commands| {
			send.try_send(()).ok();
			commands.entity(ev.observer()).despawn();
		})
		.await;
		recv.recv().await.ok();
		self
	}

	/// Handles an error using the world's default error handler.
	pub async fn handle_command_error<F>(
		&self,
		err: BevyError,
		location: &'static Location<'static>,
	) -> &Self {
		self.with_then(|world| {
			world.handle_command_error::<F>(err, location);
		})
		.await;
		self
	}
}

/// A handle for operating on a specific entity from async contexts.
/// This type is [`Copy`] as it only stores an [`Entity`] and [`WorldId`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AsyncEntity {
	entity: Entity,
	world: AsyncWorld,
}

impl AsyncEntity {
	/// Returns the entity ID.
	pub fn id(&self) -> Entity { self.entity }

	/// Returns a reference to the [`AsyncWorld`].
	pub fn world(&self) -> &AsyncWorld { &self.world }

	/// Runs a function with access to the entity.
	pub fn with(
		&self,
		func: impl 'static + Send + FnOnce(EntityWorldMut),
	) -> &Self {
		let entity = self.entity;
		self.world.with(move |world: &mut World| {
			let entity = world.entity_mut(entity);
			func(entity);
		});
		self
	}

	/// Runs a function with access to the entity and returns the result.
	pub async fn with_then<O>(
		&self,
		func: impl 'static + Send + FnOnce(EntityWorldMut) -> O,
	) -> O
	where
		O: 'static + Send + Sync,
	{
		let entity = self.entity;
		self.world
			.with_then(move |world: &mut World| {
				let entity = world.entity_mut(entity);
				func(entity)
			})
			.await
	}

	/// Spawns an async task.
	pub fn run_async<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = ()>
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with_then(move |mut entity| {
			entity.run_async(func);
		})
	}

	/// Spawns an async task on the local thread.
	pub fn run_async_local<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = ()>
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		self.with_then(move |mut entity| {
			entity.run_async_local(func);
		})
	}

	/// Gets a component and runs a function with it.
	pub async fn get<T: Component, O>(
		&self,
		func: impl 'static + Send + FnOnce(&T) -> O,
	) -> Result<O>
	where
		O: 'static + Send + Sync,
	{
		self.with_then(|entity| {
			if let Some(comp) = entity.get() {
				func(comp).xok()
			} else {
				bevybail!("Component not found: {}", std::any::type_name::<T>())
			}
		})
		.await
	}

	/// Runs a function with access to the entity id and system parameter state.
	pub fn with_state<T: 'static + SystemParam, O>(
		&self,
		func: impl 'static + Send + FnOnce(Entity, T::Item<'_, '_>) -> O,
	) -> impl Future<Output = O>
	where
		O: 'static + Send + Sync,
	{
		self.with_then(|mut entity| entity.with_state(func))
	}

	/// Gets a mutable component and runs a function with it.
	pub async fn get_mut<T: Component<Mutability = Mutable>, O>(
		&self,
		func: impl 'static + Send + FnOnce(Mut<T>) -> O,
	) -> Result<O>
	where
		O: 'static + Send + Sync,
	{
		self.with_then(|mut entity| {
			if let Some(comp) = entity.get_mut() {
				func(comp).xok()
			} else {
				bevybail!("Component not found: {}", std::any::type_name::<T>())
			}
		})
		.await
	}

	/// Gets a cloned component.
	pub async fn get_cloned<T: Component + Clone>(&self) -> Result<T> {
		self.get::<T, _>(|comp| comp.clone()).await
	}
	/// Gets two cloned components.
	pub async fn get_cloned2<T1: Component + Clone, T2: Component + Clone>(
		&self,
	) -> Result<(T1, T2)> {
		self.with_then(|entity| {
			(
				entity.try_get::<T1>()?.clone(),
				entity.try_get::<T2>()?.clone(),
			)
				.xok()
		})
		.await
	}


	/// Takes a component from the entity.
	pub async fn take<T: Component>(&self) -> Option<T> {
		self.with_then(|mut entity| entity.take()).await
	}

	/// Inserts a bundle into the entity.
	pub fn insert<B: Bundle>(&self, bundle: B) -> &Self {
		self.with(|mut entity| {
			entity.insert(bundle);
		});
		self
	}

	/// Inserts a bundle and waits for completion.
	pub async fn insert_then<B: Bundle>(&self, bundle: B) -> &Self {
		self.with_then(|mut entity| {
			entity.insert(bundle);
		})
		.await;
		self
	}

	/// Spawns a child entity and returns its ID.
	pub async fn spawn_child<B: Bundle>(&self, bundle: B) -> Entity {
		let id = self.entity;
		self.world
			.with_then(move |world| world.spawn((bundle, ChildOf(id))).id())
			.await
	}

	/// Triggers an entity event.
	pub fn trigger<'t, E: EntityEvent<Trigger<'t>: Default>>(
		&self,
		ev: impl 'static + Send + Sync + FnOnce(Entity) -> E,
	) -> &Self {
		self.with(move |mut entity| {
			entity.trigger(ev);
		});
		self
	}

	/// Triggers an entity event and waits for completion.
	pub async fn trigger_then<'t, E: EntityEvent<Trigger<'t>: Default>>(
		&self,
		ev: impl 'static + Send + Sync + FnOnce(Entity) -> E,
	) -> &Self {
		self.with_then(move |mut entity| {
			entity.trigger(ev);
		})
		.await;
		self
	}

	/// Triggers an entity target event and waits for completion.
	pub async fn trigger_target_then<M>(
		&self,
		event: impl IntoEntityTargetEvent<M>,
	) -> &Self {
		self.with_then(|mut entity| {
			entity.trigger_target(event);
		})
		.await;
		self
	}

	/// Registers an observer on the entity.
	pub async fn observe<E: Event, B: Bundle, M>(
		&self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> &Self {
		self.with_then(|mut entity| {
			entity.observe_any(observer);
		})
		.await;
		self
	}

	/// Awaits a single event of type `E` for this entity, then despawns the observer.
	pub async fn await_event<E: Event, B: Bundle>(&self) -> &Self {
		let (send, recv) = async_channel::bounded(1);
		self.observe(move |ev: On<E, B>, mut commands: Commands| {
			send.try_send(()).ok();
			commands.entity(ev.observer()).despawn();
		})
		.await;
		recv.recv().await.ok();
		self
	}

	/// Despawns the entity.
	pub async fn despawn(&self) {
		self.with_then(move |entity| {
			entity.despawn();
		})
		.await;
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
		spawn_async_task(AsyncWorld::new(self.id()), func);
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
		spawn_async_task_local(AsyncWorld::new(self.id()), func);
		self
	}

	/// Spawns an async task, flushes all async tasks and returns the output.
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
		spawn_async_task_then(
			AsyncWorld::new(self.id()),
			|| self.update_local(),
			func,
		)
	}

	/// Spawns an async local task, flushes all async tasks and returns the output.
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
		spawn_async_task_local_then(
			AsyncWorld::new(self.id()),
			|| self.update_local(),
			func,
		)
	}
}

/// Extension trait adding async command methods to [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutAsyncCommandsExt)]
pub impl EntityWorldMut<'_> {
	/// Spawns an async task for this entity.
	// Only mutable to allow for ergonomic chaining.
	#[track_caller]
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: 'static + Send + Sync + IntoResult,
	{
		let id = self.id();
		spawn_async_task_local(
			AsyncWorld::new(self.world().id()),
			move |world| func(world.entity(id)),
		);
		self
	}

	/// Spawns an async task on the local thread for this entity.
	// Only mutable to allow for ergonomic chaining.
	#[track_caller]
	fn run_async_local<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + Sync + IntoResult,
	{
		let id = self.id();
		spawn_async_task_local(
			AsyncWorld::new(self.world().id()),
			move |world| func(world.entity(id)),
		);
		self
	}

	/// Spawns an async task, flushes all async tasks and returns the output.
	#[track_caller]
	fn run_async_then<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = Out>
	where
		Func: 'static + Send + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: 'static + Send + Sync,
	{
		let id = self.id();
		spawn_async_task_then(
			AsyncWorld::new(self.world().id()),
			|| self.world_scope(World::update_local),
			move |world| func(world.entity(id)),
		)
	}

	/// Spawns an async local task, flushes all async tasks and returns the output.
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
		spawn_async_task_local_then(
			AsyncWorld::new(self.world().id()),
			|| self.world_scope(World::update_local),
			move |world| func(world.entity(id)),
		)
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::tasks::futures_lite::future;

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
				world.insert_resource(Count(0));
				world
					.with_resource_then::<Count, _>(|mut count| {
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
					.with_resource_then::<Count, _>(|mut count| {
						count.0 += 1;
					})
					.await;
			})
			.await;
		world
			.run_async_then(|world| async move {
				world
					.with_resource_then::<Count, _>(|mut count| {
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
		let result =
			app.world_mut().run_async_then(|_| future::ready(42)).await;
		result.xpect_eq(42);
	}

	// requires panic = "abort"
	// #[beet_core::test]
	#[allow(unused)]
	async fn panic_handling() {
		let mut app = test_app();
		let world = app.world_mut();
		world.insert_resource(Count(0));

		// This should not crash the test runner
		world.run_async(async |_world| -> () {
			panic!("This panic should be caught");
		});

		// Wait for the panic task to complete
		world
			.run_async_local_then(|world| async move {
				world.resource::<Count>().await
			})
			.await
			.xpect_eq(Count(0));

		// Verify the world is still functional after the panic
		world.resource_mut::<Count>().0 = 42;
		world.resource::<Count>().0.xpect_eq(42);
	}
}
