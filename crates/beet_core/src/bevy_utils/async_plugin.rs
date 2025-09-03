use async_channel;
use async_channel::TryRecvError;
use bevy::ecs::system::RunSystemOnce;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::Task;
use bevy::tasks::futures_lite::Stream;
use bevy::tasks::futures_lite::StreamExt;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// Plugin that polls background async work and applies produced CommandQueues
/// to the main Bevy world.
///
/// This unified implementation treats both single-shot futures and streams as
/// producers of `CommandQueue` items. Background tasks push `CommandQueue`
/// values into an `async_channel::unbounded` channel; the main-thread poller
/// drains those queues each frame and applies them to the world.
///
/// Futures are supported by adapting them to a single-item stream (`FutureAsStream`).
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
	mut stream_tasks: Query<(Entity, &mut AsyncStreamTask)>,
) {
	while let Ok(mut queue) = channel.rx.try_recv() {
		commands.append(&mut queue);
	}

	for (entity, task) in tasks {
		// if block_on(future::poll_once(&mut task.0)).is_some() {
		if task.0.is_finished() {
			commands.entity(entity).despawn();
		}
	}

	// Streaming handling: drain all ready command queues produced by streams
	for (entity, task) in &mut stream_tasks {
		loop {
			match task.receiver.try_recv() {
				Ok(mut queue) => {
					commands.append(&mut queue);
				}
				Err(TryRecvError::Empty) => {
					// No more items right now; keep the task for future frames
					break;
				}
				Err(TryRecvError::Closed) => {
					// Producer finished and channel is closed: remove the component
					commands.entity(entity).remove::<AsyncStreamTask>();
					break;
				}
			}
		}
	}
}

/// Streaming task: background task that sends `CommandQueue` chunks as items.
#[derive(Component)]
pub struct AsyncTask(Task<()>);

impl AsyncTask {
	/// A system to reduce boilerplate in spawing async tasks,
	/// running the provided func with an [`AsyncQueue`],
	/// returning another future resolving to its output.
	pub fn spawn_with_queue<Func, Fut, Out>(
		In(func): In<Func>,
		commands: Commands,
		channel: Res<AsyncChannel>,
	) -> Pin<Box<dyn Future<Output = Out>>>
	where
		Func: 'static + FnOnce(AsyncQueue) -> Fut,
		Fut: 'static + Future<Output = Out>,
		Out: 'static,
	{
		let tx = AsyncQueue::new(channel.tx());
		let fut = func(tx);
		Self::spawn(In(fut), commands)
	}
	/// A system to reduce boilerplate in spawing async tasks,
	/// running the provided future, returning another future resolving to its output.
	pub fn spawn<Fut, Out>(
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
			tx_out.try_send(out).expect("Failed to send output");
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
}


/// A portable channel for sending a [`CommandQueue`] to the world
#[derive(Clone)]
pub struct AsyncQueue {
	tx: async_channel::Sender<CommandQueue>,
}

impl AsyncQueue {
	pub fn new(tx: async_channel::Sender<CommandQueue>) -> Self { Self { tx } }

	pub fn spawn<B: Bundle>(&self, bundle: B) {
		self.with(move |world: &mut World| {
			world.spawn(bundle);
		});
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
		self.with(move |world: &mut World| {
			func(world.resource_mut::<R>());
		});
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
		func: impl Command + FnOnce(&mut World) -> O,
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


#[cfg(test)]
mod tests {
	use super::*;
	use bevy::tasks::futures_lite::future;
	use futures_lite::future::block_on;
	use sweet::prelude::*;

	#[test]
	fn future_as_stream_yields_single_item() {
		let fut = async { 123usize };
		let stream = FutureAsStream::new(fut);
		// Pin the stream so its `.next().await` future does not require `Unpin`.
		futures_lite::pin!(stream);
		block_on(async { stream.next().await })
			.xpect()
			.to_be(Some(123));
		block_on(async { stream.next().await }).xpect().to_be_none();
	}


	#[derive(Default, Resource)]
	struct Count(usize);

	#[sweet::test]
	async fn async_task() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		let fut = app
			.world_mut()
			.run_system_cached_with(
				AsyncTask::spawn_with_queue,
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
}
