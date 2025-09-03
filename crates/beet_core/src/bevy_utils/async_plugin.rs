use async_channel;
use beet_utils::time_ext;
use bevy::ecs::component::Mutable;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::Task;
use std::future::Future;
use std::pin::Pin;

use crate::prelude::AppExt;

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

/// Task containing futures communicating with the world via channels
#[derive(Component)]
pub struct AsyncTask(Task<()>);

impl AsyncTask {
	/// A system to reduce boilerplate in spawing async tasks, running the provided
	pub fn spawn<Fut, Out>(In(fut): In<Fut>, mut commands: Commands)
	where
		// no send requirement for std AsyncComputeTaskPool
		Fut: 'static + Future<Output = Out>,
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
		Fut: 'static + Future<Output = Out>,
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
		Fut: 'static + Future<Output = Out>,
		Out: 'static,
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
		Fut: 'static + Future<Output = Out>,
		Out: 'static,
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
		Fut: 'static + Future<Output = Result>,
	{
		let queue = channel.queue();
		let fut = func(queue.clone());
		// we can discard future, its still ran by
		// bevy tasks
		let _ = Self::spawn_then(
			In(async move {
				if let Err(err) = fut.await {
					eprintln!("Async task failed: {}", err);
					queue.send_event(AppExit::from_code(1));
				}
			}),
			commands,
		);
	}
}


#[derive(Resource)]
pub struct AsyncChannel {
	/// the sender for the async channel
	tx: async_channel::Sender<CommandQueue>,
	/// the receiver for the async channel, not accesible
	rx: async_channel::Receiver<CommandQueue>,
}

impl Default for AsyncChannel {
	fn default() -> Self {
		let (tx, rx) = async_channel::unbounded();
		Self { rx, tx }
	}
}

impl AsyncChannel {
	/// Get the sender of the channel
	pub fn tx(&self) -> async_channel::Sender<CommandQueue> { self.tx.clone() }
	pub fn queue(&self) -> AsyncQueue {
		AsyncQueue {
			tx: self.tx.clone(),
		}
	}

	/// Uses the [`AsyncChannel::rx`] as a signal to run updates,
	/// this means that the rx in poll_async_tasks should always be empty
	/// Note:
	///
	/// The async runner has an agressive poll, updating the app every 100us.
	/// For realtime apps use the regular runner.
	pub async fn runner_async(mut app: App) -> AppExit {
		app.init();
		let mut task_query = app.world_mut().query::<&mut AsyncTask>();
		let rx = app.world().resource::<AsyncChannel>().rx.clone();
		loop {
			// println!("updating..");
			app.update();
			if let Some(exit) = app.should_exit() {
				return exit;
			}
			// flush rx
			while let Ok(mut queue) = rx.try_recv() {
				app.world_mut().commands().append(&mut queue);
			}

			if task_query.query(app.world_mut()).is_empty() {
				// no current tasks, wait for the next command queue

				// println!("awaiting rx");
				// otherwise await the next rx then loop
				match rx.recv().await {
					Ok(mut queue) => {
						// println!("received queue");
						app.world_mut().commands().append(&mut queue);
					}
					Err(err) => {
						eprintln!("AsyncChannel error: {}", err);
						return AppExit::from_code(1);
					}
				}
			} else {
				// tasks are in flight, rest for a bit

				// TODO: exponential backoff 10us to 10ms
				time_ext::sleep(std::time::Duration::from_micros(100)).await;
			}
		}
	}
}

/// A portable channel for sending a [`CommandQueue`] to the world
#[derive(Clone)]
pub struct AsyncQueue {
	tx: async_channel::Sender<CommandQueue>,
}

impl AsyncQueue {
	pub fn new(tx: async_channel::Sender<CommandQueue>) -> Self { Self { tx } }

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

	pub fn trigger<E: Event>(&self, event: E) {
		self.with(move |world| {
			world.trigger(event);
		});
	}
	pub fn send_event<E: Event>(&self, event: E) {
		self.with(move |world| {
			world.send_event(event);
		});
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
	pub fn with(&self, func: impl 'static + Send + FnOnce(EntityWorldMut)) {
		let entity = self.entity;
		self.queue.with(move |world: &mut World| {
			let entity = world.entity_mut(entity);
			func(entity);
		});
	}
	pub fn get_mut<T: Component<Mutability = Mutable>>(
		&self,
		func: impl 'static + Send + FnOnce(Mut<T>),
	) -> &Self {
		self.with(|mut entity| {
			let comp = entity.get_mut().unwrap();
			func(comp);
		});
		self
	}

	pub fn trigger<E: Event>(&self, event: E) -> &Self {
		self.with(|mut entity| {
			entity.trigger(event);
		});
		self
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::AppExitExt;

	use super::*;
	use beet_utils::time_ext;
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
		fut.await.xpect().to_be(32);

		// queue not yet applied
		app.world_mut().resource::<Count>().0.xpect().to_be(0);

		app.update();

		// queue now applied
		app.world_mut().resource::<Count>().0.xpect().to_be(1);
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
					assert_eq!(next, 1);
					time_ext::sleep(std::time::Duration::from_millis(2)).await;
					let next = queue
						.update_resource_then(|mut res: Mut<Count>| {
							res.0 += 1;
							res.0
						})
						.await;
					assert_eq!(next, 2);

					future::yield_now().await;
					queue.send_event(AppExit::Success);
					queue.resource::<Count>().await.0
				},
			)
			.unwrap();
		// must update app first or future will hang
		app.run_async(AsyncChannel::runner_async).await;

		// future completed
		fut.await.xpect().to_be(2);
	}
	#[sweet::test]
	async fn results() {
		use crate::bevybail;

		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		app.world_mut()
			.run_system_cached_with(
				AsyncTask::spawn_with_queue_unwrap,
				async |_| {
					time_ext::sleep(std::time::Duration::from_millis(2)).await;
					// future::yield_now().await;
					bevybail!("intentional error")
				},
			)
			.unwrap();
		app.run_async(AsyncChannel::runner_async)
			.await
			.into_result()
			.xpect()
			.to_be_err();
	}
}
