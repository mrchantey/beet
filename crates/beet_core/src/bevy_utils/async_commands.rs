use crate::prelude::*;
use async_channel;
use async_channel::Receiver;
use async_channel::Sender;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::CommandQueue;
use bevy::tasks::IoTaskPool;
use bevy::tasks::Task;
use std::future::Future;

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
pub struct AsyncPlugin;

impl Plugin for AsyncPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<AsyncChannel>()
			.add_systems(PreUpdate, poll_async_tasks);
	}
}

fn poll_async_tasks(
	mut commands: Commands,
	channel: Res<AsyncChannel>,
	tasks: Query<(Entity, &AsyncTask)>,
) -> Result {
	// 1. remove any completed tasks
	for (entity, task) in tasks {
		if task.0.is_finished() {
			commands.entity(entity).despawn();
		}
	}
	// 2. run all ready queues
	while let Ok(mut queue) = channel.rx.try_recv() {
		commands.append(&mut queue);
	}
	Ok(())
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
	pub fn run<Func, Fut, Out>(&mut self, func: Func) -> Entity
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		let world = self.channel.world();
		let world2 = world.clone();
		let task = IoTaskPool::get()
			.spawn(async move { func(world).await.apply(world2) });
		self.commands.spawn(AsyncTask(task)).id()
	}
	/// Spawn an async task, returing the spawned entity containing the [`AsyncTask`]
	pub fn run_local<Func, Fut, Out>(&mut self, func: Func) -> Entity
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		let world = self.channel.world();
		let world2 = world.clone();
		let task = IoTaskPool::get()
			.spawn_local(async move { func(world).await.apply(world2) });
		self.commands.spawn(AsyncTask(task)).id()
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

/// Task containing futures communicating with the world via channels
#[derive(Component, Deref, DerefMut)]
pub struct AsyncTask(Task<()>);

/// Contains the channel used by async functions to send [`CommandQueue`]s
#[derive(Resource)]
pub struct AsyncChannel {
	/// the sender for the async channel
	tx: Sender<CommandQueue>,
	/// the receiver for the async channel, not accesible
	pub(super) rx: Receiver<CommandQueue>,
}

impl Default for AsyncChannel {
	fn default() -> Self {
		let (tx, rx) = async_channel::unbounded();
		Self { rx, tx }
	}
}

impl AsyncChannel {
	/// Get the sender of the channel
	pub fn tx(&self) -> Sender<CommandQueue> { self.tx.clone() }
	pub fn world(&self) -> AsyncWorld {
		AsyncWorld {
			tx: self.tx.clone(),
		}
	}
}

/// A portable channel for sending a [`CommandQueue`] to the world
#[derive(Clone)]
pub struct AsyncWorld {
	tx: Sender<CommandQueue>,
}

impl AsyncWorld {
	pub fn new(tx: Sender<CommandQueue>) -> Self { Self { tx } }

	/// Queues the command
	pub fn with(&self, func: impl Command + FnOnce(&mut World)) {
		let mut queue = CommandQueue::default();
		queue.push(func);
		self.tx.try_send(queue).unwrap();
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
		self.tx.try_send(queue).unwrap();
		async move { out_rx.recv().await.unwrap() }
	}

	pub fn entity(&self, entity: Entity) -> AsyncEntity {
		AsyncEntity {
			entity,
			queue: self.clone(),
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
	) -> impl Future<Output = Entity> {
		self.with_then(move |world: &mut World| world.spawn(bundle).id())
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

	pub fn run_system_cached<O, M, S>(&self, system: S)
	where
		O: 'static,
		S: 'static + Send + IntoSystem<(), O, M>,
	{
		self.run_system_cached_with(system, ());
	}
	pub fn run_system_cached_with<I, O, M, S>(
		&self,
		system: S,
		input: I::Inner<'_>,
	) where
		I: SystemInput + 'static,
		for<'a> I::Inner<'a>: 'static + Send + Sync,
		O: 'static,
		S: 'static + Send + IntoSystem<I, O, M>,
	{
		self.with(move |world| {
			world.run_system_cached_with(system, input).ok();
		});
	}
	/// Spawn an async task, returing the spawned entity containing the [`AsyncTask`]
	pub fn run_async<Func, Fut, Out>(
		&self,
		func: Func,
	) -> impl Future<Output = Entity>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		self.with_then(move |world| world.run_async(func).id())
	}
	/// Spawn an async task, returing the spawned entity containing the [`AsyncTask`]
	pub fn run_async_local<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> impl Future<Output = Entity>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		self.with_then(move |world| world.run_async_local(func).id())
	}
}


pub struct AsyncEntity {
	entity: Entity,
	queue: AsyncWorld,
}

impl AsyncEntity {
	pub async fn with(
		&self,
		func: impl 'static + Send + FnOnce(EntityWorldMut),
	) -> &Self {
		let entity = self.entity;
		self.queue
			.with_then(move |world: &mut World| {
				let entity = world.entity_mut(entity);
				func(entity);
			})
			.await;
		self
	}
	pub async fn get_mut<T: Component<Mutability = Mutable>>(
		&self,
		func: impl 'static + Send + FnOnce(Mut<T>),
	) -> &Self {
		self.with(|mut entity| {
			let comp = entity.get_mut().unwrap();
			func(comp);
		})
		.await
	}

	pub async fn insert<B: Bundle>(&self, component: B) -> &Self {
		self.with(|mut entity| {
			entity.insert(component);
		})
		.await
	}

	pub async fn trigger<'t, E: EntityEvent<Trigger<'t>: Default>>(
		&self,
		ev: impl 'static + Send + Sync + FnOnce(Entity) -> E,
	) -> &Self {
		self.with(move |mut entity| {
			entity.trigger(ev);
		})
		.await
	}
	pub async fn trigger_target<M>(
		&self,
		event: impl IntoEntityTargetEvent<M>,
	) -> &Self {
		self.with(|mut entity| {
			entity.trigger_target(event);
		})
		.await
	}

	pub async fn observe<E: Event, B: Bundle, M>(
		&self,
		observer: impl IntoObserverSystem<E, B, M>,
	) -> &Self {
		self.with(|mut entity| {
			entity.observe_any(observer);
		})
		.await;
		self
	}

	pub async fn despawn(&self) {
		self.with(move |entity| {
			entity.despawn();
		})
		.await;
	}
}

#[extend::ext(name=WorldAsyncCommandsExt)]
pub impl World {
	fn run_async<Func, Fut, Out>(&mut self, func: Func) -> EntityWorldMut<'_>
	where
		Func: 'static + Send + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out> + Send,
		Out: AsyncTaskOut,
	{
		let world = self.resource::<AsyncChannel>().world();
		let world2 = world.clone();
		let task = IoTaskPool::get()
			.spawn(async move { func(world).await.apply(world2) });
		self.spawn(AsyncTask(task))
	}
	fn run_async_local<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> EntityWorldMut<'_>
	where
		Func: 'static + FnOnce(AsyncWorld) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		let world = self.resource::<AsyncChannel>().world();
		let world2 = world.clone();
		let task = IoTaskPool::get()
			.spawn_local(async move { func(world).await.apply(world2) });
		self.spawn(AsyncTask(task))
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
		let world = self.resource::<AsyncChannel>().world();
		let (send, recv) = async_channel::bounded(1);
		let task = IoTaskPool::get().spawn(async move {
			let out = func(world).await;
			let _ = send.try_send(out);
		});
		self.spawn(AsyncTask(task));
		async move {
			AsyncRunner::flush_async_tasks(self).await;
			recv.recv().await.unwrap()
		}
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
		let world = self.resource::<AsyncChannel>().world();
		let (send, recv) = async_channel::bounded(1);
		let task = IoTaskPool::get().spawn_local(async move {
			let out = func(world).await;
			let _ = send.try_send(out);
		});
		self.spawn(AsyncTask(task));
		async move {
			AsyncRunner::flush_async_tasks(self).await;
			recv.recv().await.unwrap()
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use bevy::tasks::futures_lite::future;
	use sweet::prelude::*;


	#[derive(Default, Resource, Clone)]
	struct Count(usize);

	#[sweet::test]
	async fn async_task() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		app.world_mut().run_async_local(async |world| {
			let next = 1;
			future::yield_now().await;
			world.with_resource::<Count>(move |mut count| count.0 += next);
			future::yield_now().await;
		});

		// world not yet applied
		app.world_mut().resource::<Count>().0.xpect_eq(0);

		AsyncRunner::flush_async_tasks(app.world_mut()).await;

		// world now applied
		app.world_mut().resource::<Count>().0.xpect_eq(1);
	}
	#[sweet::test]
	async fn async_queue() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		// time_ext::sleep !send in wasm
		app.world_mut().run_async_local(async |world| {
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
		});

		// must update app first or future will hang
		AsyncRunner::flush_async_tasks(app.world_mut()).await;

		app.world_mut().resource::<Count>().0.xpect_eq(2);
	}
	#[sweet::test]
	async fn results() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		app.set_error_handler(|_, _| {});
		app.world_mut().run_async_local(async |_| {
			time_ext::sleep(Duration::from_millis(2)).await;
			bevybail!("intentional error")
		});
		app.run_async().await.into_result().xpect_err();
	}
	#[sweet::test]
	async fn run_async_then() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		app.world_mut()
			.run_async_then(async |_| 32)
			.await
			.xpect_eq(32);
	}
}
