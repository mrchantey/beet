use crate::prelude::*;
use async_channel;
use async_channel::Receiver;
use async_channel::Sender;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::system::RegisteredSystemError;
use bevy::ecs::system::RunSystemError;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::CommandQueue;
use bevy::tasks::IoTaskPool;
use std::future::Future;

/// In wasm an single threaded environment, wraps this type in a [`SendWrapper`],
/// otherwise is just the type itself
#[cfg(all(feature = "multi_threaded", not(target_arch = "wasm32")))]
pub type MaybeSendWrapper<T> = send_wrapper::SendWrapper<T>;
/// In wasm an single threaded environment, wraps this type in a [`SendWrapper`],
/// otherwise is just the type itself
#[cfg(not(all(feature = "multi_threaded", not(target_arch = "wasm32"))))]
pub type MaybeSendWrapper<T> = T;

pub fn maybe_send_wrapper<T>(value: T) -> MaybeSendWrapper<T> {
	#[cfg(all(feature = "multi_threaded", not(target_arch = "wasm32")))]
	{
		send_wrapper::SendWrapper::new(value)
	}
	#[cfg(not(all(feature = "multi_threaded", not(target_arch = "wasm32"))))]
	{
		value
	}
}

#[cfg(all(feature = "multi_threaded", not(target_arch = "wasm32")))]
pub trait MaybeSend: Send {}
#[cfg(not(all(feature = "multi_threaded", not(target_arch = "wasm32"))))]
pub trait MaybeSend {}
#[cfg(all(feature = "multi_threaded", not(target_arch = "wasm32")))]
impl<T> MaybeSend for T where T: Send {}
#[cfg(not(all(feature = "multi_threaded", not(target_arch = "wasm32"))))]
impl<T> MaybeSend for T {}
#[cfg(all(feature = "multi_threaded", not(target_arch = "wasm32")))]
pub trait MaybeSync: Sync {}
#[cfg(not(all(feature = "multi_threaded", not(target_arch = "wasm32"))))]
pub trait MaybeSync {}
#[cfg(all(feature = "multi_threaded", not(target_arch = "wasm32")))]
impl<T> MaybeSync for T where T: Sync {}
#[cfg(not(all(feature = "multi_threaded", not(target_arch = "wasm32"))))]
impl<T> MaybeSync for T {}


/// Plugin that polls background async work and applies produced CommandQueues
/// to the main Bevy world.
/// This plugin will init the [`TaskPoolPlugin`] if unintialized,
/// so must be added after [`DefaultPlugins`] / [`MinimalPlugins`]
#[derive(Default)]
pub struct AsyncPlugin;

impl Plugin for AsyncPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<TaskPoolPlugin>()
			.init_resource::<AsyncChannel>()
			.add_systems(PreUpdate, append_async_queues);
	}
}
/// Append all [`AsyncChannel::rx`]
fn append_async_queues(
	mut commands: Commands,
	channel: Res<AsyncChannel>,
) -> Result {
	while let Ok(mut queue) = channel.rx.try_recv() {
		commands.append(&mut queue);
	}
	Ok(())
}

fn spawn_async_task<Func, Fut, Out>(world: AsyncWorld, func: Func)
where
	Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + MaybeSend + Future<Output = Out>,
	Out: AsyncTaskOut,
{
	let world2 = world.clone();
	let world3 = world.clone();
	IoTaskPool::get()
		.spawn(async move {
			world3
				.with_resource_then::<AsyncChannel, _>(|mut channel| {
					channel.increment_tasks();
				})
				.await;
			func(world).await.apply(world2);
			world3
				.with_resource_then::<AsyncChannel, _>(|mut channel| {
					channel.decrement_tasks();
				})
				.await;
		})
		.detach();
}
fn spawn_async_task_local<Func, Fut, Out>(world: AsyncWorld, func: Func)
where
	Func: 'static + FnOnce(AsyncWorld) -> Fut,
	Fut: 'static + Future<Output = Out>,
	Out: AsyncTaskOut,
{
	let world2 = world.clone();
	let world3 = world.clone();
	IoTaskPool::get()
		.spawn_local(async move {
			world3
				.with_resource_then::<AsyncChannel, _>(|mut channel| {
					channel.increment_tasks();
				})
				.await;
			func(world).await.apply(world2);
			world3
				.with_resource_then::<AsyncChannel, _>(|mut channel| {
					channel.decrement_tasks();
				})
				.await;
		})
		.detach();
}

/// Spawn the async task, flush all async tasks and return the output
fn spawn_async_task_then<Func, Fut, Out>(
	world: AsyncWorld,
	update: impl FnMut(),
	func: Func,
) -> impl Future<Output = Out>
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
}
/// Spawn the async local task, flush all async tasks and return the output
fn spawn_async_task_local_then<Func, Fut, Out>(
	world: AsyncWorld,
	update: impl FnMut(),
	func: Func,
) -> impl Future<Output = Out>
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
	AsyncRunner::poll_and_update(update, recv)
}


/// Commands used to run async functions, passing in an [`AsyncWorld`] which
/// can be used to send and received values from the [`World`]
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
///
/// #[derive(Clone, Resource)]
/// struct MyResource(u32);
///
/// fn my_system(mut commands: AsyncCommands){
/// 	commands.run(async |world|{
///			world.insert_resource(MyResource(2));
///			let value = world.resource::<MyResource>().await.0;
///			assert_eq!(value, 2);
/// 	});
/// }
/// ```
#[derive(SystemParam)]
pub struct AsyncCommands<'w, 's> {
	/// The commands used for spawning an entity with a [`AsyncTask`]
	pub commands: Commands<'w, 's>,
	/// The channel used to create an [`AsyncWorld`] passed to the async function
	pub channel: Res<'w, AsyncChannel>,
}


impl AsyncCommands<'_, '_> {
	/// Spawn an async task, returing the spawned entity containing the [`AsyncTask`]
	pub fn run<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		spawn_async_task(self.channel.world(), func);
	}
	/// Spawn an async task, returing the spawned entity containing the [`AsyncTask`]
	pub fn run_local<Func, Fut, Out>(&mut self, func: Func)
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		spawn_async_task_local(self.channel.world(), func);
	}
}

pub trait AsyncTaskOut: 'static + Send + Sync {
	fn apply(self, world: AsyncWorld);
}
impl AsyncTaskOut for () {
	fn apply(self, _: AsyncWorld) {}
}


impl AsyncTaskOut for Result {
	fn apply(self, world: AsyncWorld) {
		match self {
			Ok(_) => {}
			Err(e) => {
				bevy::log::error!("Async task error: {:#}", e);
				world.write_message(AppExit::error());
			}
		}
	}
}

/// Contains the channel used by async functions to send [`CommandQueue`]s
#[derive(Resource)]
pub struct AsyncChannel {
	/// the number of tasks currently in flight
	task_count: usize,
	/// the sender for the async channel
	tx: Sender<CommandQueue>,
	/// the receiver for the async channel, not accesible
	rx: Receiver<CommandQueue>,
}

impl Default for AsyncChannel {
	fn default() -> Self {
		let (tx, rx) = async_channel::unbounded();
		Self {
			rx,
			tx,
			task_count: 0,
		}
	}
}

impl AsyncChannel {
	pub fn task_count(&self) -> usize { self.task_count }
	/// Get the sender of the channel
	pub fn tx(&self) -> Sender<CommandQueue> { self.tx.clone() }
	pub fn world(&self) -> AsyncWorld {
		AsyncWorld {
			tx: self.tx.clone(),
		}
	}
	fn increment_tasks(&mut self) -> &mut Self {
		self.task_count += 1;
		self
	}
	fn decrement_tasks(&mut self) -> &mut Self {
		self.task_count = self.task_count.saturating_sub(1);
		self
	}
}

/// A portable channel for sending a [`CommandQueue`] to the world
#[derive(Clone)]
pub struct AsyncWorld {
	tx: Sender<CommandQueue>,
}

impl AsyncWorld {
	pub fn new(tx: Sender<CommandQueue>) -> Self { Self { tx } }

	fn send(&self, queue: CommandQueue) {
		if let Err(err) = self.tx.try_send(queue) {
			warn!("Failed to send command queue: {}", err);
		}
	}
	/// Queues the command
	pub fn with(&self, func: impl Command + FnOnce(&mut World)) {
		let mut queue = CommandQueue::default();
		queue.push(func);
		self.send(queue);
	}
	/// Queues the command, creating another channel that will resolve when
	/// the task is complete, returing its output
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
			out_rx
				.recv()
				.await
				.expect("channel closed, was the world dropped?")
		}
	}

	pub fn entity(&self, entity: Entity) -> AsyncEntity {
		AsyncEntity {
			entity,
			world: self.clone(),
		}
	}

	pub fn spawn<B: Bundle>(&self, bundle: B) {
		self.with(move |world: &mut World| {
			world.spawn(bundle);
		});
	}
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

	pub fn insert_resource<R: Resource>(&self, resource: R) {
		self.with(move |world: &mut World| {
			world.insert_resource(resource);
		});
	}
	pub fn with_resource<R: Resource>(
		&self,
		func: impl FnOnce(Mut<R>) + Send + 'static,
	) {
		self.with(move |world| {
			func(world.resource_mut::<R>());
		});
	}
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


	/// Clone a resource and return it
	pub fn resource<R: Resource + Clone>(&self) -> impl Future<Output = R>
	where
		R: Resource,
	{
		self.with_then(move |world| world.resource::<R>().clone())
	}

	pub fn trigger<'a, E: Event<Trigger<'a>: Default>>(&self, event: E) {
		self.with(move |world| {
			world.trigger(event);
		});
	}

	pub fn write_message<E: Message>(&self, event: E) {
		self.with(move |world| {
			world.write_message(event);
		});
	}
	pub fn write_message_batch<E: Message>(
		&self,
		event: impl 'static + Send + IntoIterator<Item = E>,
	) {
		self.with(move |world| {
			world.write_message_batch(event);
		});
	}

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
	/// Spawn an async task
	pub fn run_async<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = ()>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		self.with_then(move |world| {
			world.run_async(func);
		})
	}
	/// Spawn an async task, returing the spawned entity containing the [`AsyncTask`]
	pub fn run_async_local<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = ()>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		self.with_then(move |world| {
			world.run_async_local(func);
		})
	}
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

	/// Await a single event of type E, then despawn the observer
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
}

#[derive(Clone)]
pub struct AsyncEntity {
	entity: Entity,
	world: AsyncWorld,
}

impl AsyncEntity {
	pub fn id(&self) -> Entity { self.entity }
	pub fn world(&self) -> AsyncWorld { self.world.clone() }

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
				bevybail!("Component not found")
			}
		})
		.await
	}

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
				bevybail!("Component not found")
			}
		})
		.await
	}

	pub async fn get_cloned<T: Component + Clone>(&self) -> Result<T> {
		self.get::<T, _>(|comp| comp.clone()).await
	}

	pub async fn insert<B: Bundle>(&self, bundle: B) -> &Self {
		self.with_then(|mut entity| {
			entity.insert(bundle);
		})
		.await;
		self
	}
	/// Spawn a child and return its id
	pub async fn spawn_child<B: Bundle>(&self, bundle: B) -> Entity {
		let id = self.entity;
		self.world
			.with_then(move |world| world.spawn((bundle, ChildOf(id))).id())
			.await
	}

	pub async fn trigger<'t, E: EntityEvent<Trigger<'t>: Default>>(
		&self,
		ev: impl 'static + Send + Sync + FnOnce(Entity) -> E,
	) -> &Self {
		self.with_then(move |mut entity| {
			entity.trigger(ev);
		})
		.await;
		self
	}
	pub async fn trigger_target<M>(
		&self,
		event: impl IntoEntityTargetEvent<M>,
	) -> &Self {
		self.with_then(|mut entity| {
			entity.trigger_target(event);
		})
		.await;
		self
	}

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
	/// Await a single event of type E for this entity, then despawn the observer
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

	pub async fn despawn(&self) {
		self.with_then(move |entity| {
			entity.despawn();
		})
		.await;
	}
}

#[extend::ext(name=WorldAsyncCommandsExt)]
pub impl World {
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		spawn_async_task(self.resource::<AsyncChannel>().world(), func);
		self
	}
	fn run_async_local<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		spawn_async_task_local(self.resource::<AsyncChannel>().world(), func);
		self
	}
	/// Spawn the async task, flush all async tasks and return the output
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
			self.resource::<AsyncChannel>().world(),
			|| self.update(),
			func,
		)
	}
	/// Spawn the async local task, flush all async tasks and return the output
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
			self.resource::<AsyncChannel>().world(),
			|| self.update(),
			func,
		)
	}
}
#[extend::ext(name=EntityWorldMutAsyncCommandsExt)]
pub impl EntityWorldMut<'_> {
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		spawn_async_task_local(self.resource::<AsyncChannel>().world(), func);
		self
	}
	fn run_async_local<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		spawn_async_task_local(self.resource::<AsyncChannel>().world(), func);
		self
	}
	/// Spawn the async task, flush all async tasks and return the output
	fn run_async_then<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = Out>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: 'static + Send + Sync,
	{
		spawn_async_task_then(
			self.resource::<AsyncChannel>().world(),
			|| self.world_scope(World::update),
			func,
		)
	}
	/// Spawn the async local task, flush all async tasks and return the output
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
			self.resource::<AsyncChannel>().world(),
			|| self.world_scope(World::update),
			func,
		)
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::tasks::futures_lite::future;
	use sweet::prelude::*;


	#[derive(Default, Resource, Clone)]
	struct Count(usize);

	#[sweet::test]
	async fn async_task() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Count>();
		world
			.run_async_local_then(async |world| {
				let next = 1;
				future::yield_now().await;
				world.with_resource::<Count>(move |mut count| count.0 += next);
				future::yield_now().await;
			})
			.await;

		// world not yet applied
		world.resource::<Count>().0.xpect_eq(0);

		AsyncRunner::flush_async_tasks(&mut world).await;

		// world now applied
		world.resource::<Count>().0.xpect_eq(1);
	}
	#[sweet::test]
	async fn async_queue() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Count>();
		// time_ext::sleep !send in wasm
		world
			.run_async_local_then(async |world| {
				let next = world
					.with_resource_then(|mut res: Mut<Count>| {
						res.0 += 1;
						res.0
					})
					.await;
				next.xpect_eq(1);
				time_ext::sleep(Duration::from_millis(2)).await;
				let next = world
					.with_resource_then(|mut res: Mut<Count>| {
						res.0 += 1;
						res.0
					})
					.await;
				next.xpect_eq(2);

				future::yield_now().await;
			})
			.await;

		world.resource::<Count>().0.xpect_eq(2);
	}
	#[sweet::test]
	async fn results() {
		// use bevy::ecs::error::DefaultErrorHandler;
		let mut app = App::new();
		app.add_plugins(AsyncPlugin);
		// let mut world = (MinimalPlugins, AsyncPlugin).into_world();
		let world = app.world_mut();
		world.init_resource::<Count>();
		// world.insert_resource(DefaultErrorHandler(bevy::ecs::error::ignore));
		world.run_async_local(async |_| {
			time_ext::sleep(Duration::from_millis(2)).await;
			bevybail!("intentional error")
		});

		// AsyncRunner::flush_async_tasks(&mut world).await;

		app.run_async().await.into_result().xpect_err();
	}

	#[sweet::test]
	async fn run_async_then() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Count>();
		world.run_async_then(async |_| 32).await.xpect_eq(32);
	}
}
