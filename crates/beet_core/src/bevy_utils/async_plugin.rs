use crate::prelude::*;
use async_channel;
use async_channel::Receiver;
use async_channel::Sender;
use bevy::ecs::component::Mutable;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::CommandQueue;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::Task;
use std::future::Future;
use std::pin::Pin;

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
) {
	// 1. remove any completed tasks
	for (entity, task) in tasks {
		// if block_on(future::poll_once(&mut task.0)).is_some() {
		if task.0.is_finished() {
			commands.entity(entity).despawn();
		}
	}
	// 2. run all ready queues
	while let Ok(mut queue) = channel.rx.try_recv() {
		commands.append(&mut queue);
	}
}

// TODO use this instead of AsyncTask
#[derive(SystemParam)]
pub struct AsyncCommands<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub channel: Res<'w, AsyncChannel>,
}
impl AsyncCommands<'_, '_> {
	pub fn run() {}
}

/// Task containing futures communicating with the world via channels
#[derive(Component)]
pub struct AsyncTask(Task<()>);

impl AsyncTask {
	/// A system to reduce boilerplate in spawing async tasks, running the provided
	pub fn spawn<Fut, Out>(In(fut): In<Fut>, mut commands: Commands)
	where
		// no send requirement for std AsyncComputeTaskPool
		Fut: 'static + Future<Output = Out> + MaybeSend,
	{
		let task = AsyncComputeTaskPool::get().spawn(async move {
			let _ = fut.await;
		});
		commands.spawn(Self(task));
	}
	/// A system to reduce boilerplate in spawing async tasks,
	/// running the provided func with an [`AsyncQueue`],
	///
	/// ## Warning
	///
	/// If awaiting results from methods like [`AsyncQueue::with`],
	/// the [`AsyncChannel`] must be flushed first:
	/// - For realtime apps, this will naturally occur via [`App::run`]
	/// - For tests and reactive apps, use a pattern like [`AsyncChannel::runner_async`]
	pub fn spawn_with_queue<Func, Fut, Out>(
		In(func): In<Func>,
		commands: Commands,
		channel: Res<AsyncChannel>,
	) where
		Func: 'static + FnOnce(AsyncQueue) -> Fut,
		Fut: 'static + Future<Output = Out> + MaybeSend,
		Out: 'static,
	{
		let fut = func(channel.queue());
		Self::spawn(In(fut), commands)
	}

	/// A system to reduce boilerplate in spawing async tasks,
	/// running the provided future, returning another future resolving to its output.
	/// ## Returned Future
	/// The returned future is for the *receiving channel* of the output value,
	/// not the execution of the future itsself.
	/// if you dont need the output value this future can be safely dropped and the system
	/// will still run.
	pub fn spawn_then<Fut, Out>(
		In(fut): In<Fut>,
		mut commands: Commands,
	) -> Pin<Box<dyn Future<Output = Out>>>
	where
		// no send requirement for std AsyncComputeTaskPool
		Fut: 'static + Future<Output = Out> + MaybeSend,
		Out: 'static + MaybeSend,
	{
		// channel for the final output
		let (tx_out, rx_out) = async_channel::bounded::<Out>(1);

		let task = AsyncComputeTaskPool::get().spawn(async move {
			let out = fut.await;
			tx_out.try_send(out)
				.ok(/* user dropped the output rx, thats fine */);
		});
		commands.spawn(Self(task));

		Box::pin(async move {
			match rx_out.recv().await {
				Ok(v) => v,
				Err(_) => {
					panic!("output channel closed");
				}
			}
		})
	}

	/// A system to reduce boilerplate in spawing async tasks,
	/// running the provided func with an [`AsyncQueue`],
	/// returning another future resolving to its output.
	///
	/// ## Warning
	///
	/// If awaiting results from methods like [`AsyncQueue::with`],
	/// the [`AsyncChannel`] must be flushed first:
	/// - For realtime apps, this will naturally occur via [`App::run`]
	/// - For tests and reactive apps, use a pattern like [`AsyncChannel::runner_async`]
	pub fn spawn_with_queue_then<Func, Fut, Out>(
		In(func): In<Func>,
		commands: Commands,
		channel: Res<AsyncChannel>,
	) -> Pin<Box<dyn Future<Output = Out>>>
	where
		Func: 'static + FnOnce(AsyncQueue) -> Fut,
		Fut: 'static + Future<Output = Out> + MaybeSend,
		Out: 'static + MaybeSend,
	{
		let fut = func(channel.queue());
		Self::spawn_then(In(fut), commands)
	}
	pub fn spawn_with_queue_unwrap<Func, Fut>(
		In(func): In<Func>,
		commands: Commands,
		channel: Res<AsyncChannel>,
	) where
		Func: 'static + FnOnce(AsyncQueue) -> Fut,
		Fut: 'static + Future<Output = Result> + MaybeSend,
	{
		let queue = channel.queue();
		let fut = func(queue.clone());
		// we can discard future, its still ran by
		// bevy tasks
		let _ = Self::spawn_then(
			In(async move {
				if let Err(err) = fut.await {
					eprintln!("Async task failed: {}", err);
					queue.write_message(AppExit::from_code(1));
				}
			}),
			commands,
		);
	}
}


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
	pub fn queue(&self) -> AsyncQueue {
		AsyncQueue {
			tx: self.tx.clone(),
		}
	}
}

/// A portable channel for sending a [`CommandQueue`] to the world
#[derive(Clone)]
pub struct AsyncQueue {
	tx: Sender<CommandQueue>,
}

impl AsyncQueue {
	pub fn new(tx: Sender<CommandQueue>) -> Self { Self { tx } }

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
	pub fn update_resource<R: Resource>(
		&self,
		func: impl FnOnce(Mut<R>) + Send + 'static,
	) {
		self.with(move |world| {
			func(world.resource_mut::<R>());
		});
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

	pub fn update_resource_then<R, O>(
		&self,
		func: impl 'static + Send + FnOnce(Mut<R>) -> O,
	) -> impl Future<Output = O>
	where
		R: Resource,
		O: 'static + Send + Sync,
	{
		self.with_then(move |world| func(world.resource_mut::<R>()))
	}

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
			out_tx.try_send(out).unwrap();
		});
		self.tx.try_send(queue).unwrap();
		async move { out_rx.recv().await.unwrap() }
	}
}


pub struct AsyncEntity {
	entity: Entity,
	queue: AsyncQueue,
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

	pub async fn trigger<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Traversal<E>,
	>(
		&self,
		event: E,
	) -> &Self {
		self.with(|mut entity| {
			entity.trigger_target(event);
		})
		.await
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
		let fut = app
			.world_mut()
			.run_system_cached_with(
				AsyncTask::spawn_with_queue_then,
				async |queue| {
					let next = 1;
					future::yield_now().await;
					queue.update_resource::<Count>(move |mut count| {
						count.0 += next
					});
					future::yield_now().await;
					32
				},
			)
			.unwrap();

		// future completed
		fut.await.xpect_eq(32);

		// queue not yet applied
		app.world_mut().resource::<Count>().0.xpect_eq(0);

		app.update();

		// queue now applied
		app.world_mut().resource::<Count>().0.xpect_eq(1);
	}
	#[sweet::test]
	async fn async_queue() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		let fut = app
			.world_mut()
			.run_system_cached_with(
				AsyncTask::spawn_with_queue_then,
				async |queue| {
					let next = queue
						.update_resource_then(|mut res: Mut<Count>| {
							res.0 += 1;
							res.0
						})
						.await;
					next.xpect_eq(1);
					time_ext::sleep(Duration::from_millis(2)).await;
					let next = queue
						.update_resource_then(|mut res: Mut<Count>| {
							res.0 += 1;
							res.0
						})
						.await;
					next.xpect_eq(2);

					future::yield_now().await;
					queue.resource::<Count>().await.0
				},
			)
			.unwrap();

		// must update app first or future will hang
		AsyncRunner::flush_async_tasks(app.world_mut()).await;

		// future completed
		fut.await.xpect_eq(2);
	}
	#[sweet::test]
	async fn results() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		app.world_mut()
			.run_system_cached_with(
				AsyncTask::spawn_with_queue_unwrap,
				async |_| {
					time_ext::sleep(Duration::from_millis(2)).await;
					// future::yield_now().await;
					bevybail!("intentional error")
				},
			)
			.unwrap();
		app.run_async().await.into_result().xpect_err();
	}
}
