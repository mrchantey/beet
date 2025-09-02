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


/// Streaming task: background task that sends `CommandQueue` chunks as items.
#[derive(Component)]
pub struct AsyncStreamTask {
	receiver: async_channel::Receiver<CommandQueue>,
	/// We use the receiver to detect completion but hold onto task
	/// as it cancels on drop
	_task: Task<()>,
}

impl AsyncStreamTask {
	#[allow(dead_code)]
	pub(crate) fn new(
		receiver: async_channel::Receiver<CommandQueue>,
		task: Task<()>,
	) -> Self {
		Self {
			receiver,
			_task: task,
		}
	}
}

/// Adapter that views a `Future` as a `Stream` that yields a single item.
///
/// This allows us to unify the runtime API: both `Future` and `Stream` values
/// can be scheduled through the same `spawn_for_each_stream` entry points.
pub struct FutureAsStream<Fut>(Option<Fut>);

impl<Fut> FutureAsStream<Fut> {
	pub fn new(fut: Fut) -> Self { Self(Some(fut)) }
}

impl<Fut> Stream for FutureAsStream<Fut>
where
	Fut: Future,
{
	type Item = Fut::Output;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let this = unsafe { self.get_unchecked_mut() };
		if let Some(fut) = this.0.as_mut() {
			// Safety: `fut` will not be moved after being pinned here.
			let fut = unsafe { Pin::new_unchecked(fut) };
			match fut.poll(cx) {
				Poll::Ready(v) => {
					this.0 = None;
					Poll::Ready(Some(v))
				}
				Poll::Pending => Poll::Pending,
			}
		} else {
			Poll::Ready(None)
		}
	}
}

#[derive(SystemParam)]
pub struct AsyncCommands<'w, 's> {
	pub commands: Commands<'w, 's>,
	pub channel: Res<'w, AsyncChannel>,
}

impl AsyncCommands<'_, '_> {
	pub fn spawn<Func, Fut, Out>(
		&mut self,
		func: Func,
	) -> Pin<Box<dyn Future<Output = Out>>>
	where
		Func: 'static + Fn(AsyncQueue) -> Fut,
		Fut: Future<Output = Out>,
		Out: 'static,
	{
		// channel for the final output
		let (tx_out, rx_out) = async_channel::bounded::<Out>(1);
		let tx_queue = self.channel.tx();

		let task = AsyncComputeTaskPool::get().spawn(async move {
			let out = func(AsyncQueue::new(tx_queue)).await;
			tx_out.try_send(out).expect("Failed to send output");
		});
		self.commands.spawn(AsyncTask(task));

		Box::pin(async move {
			match rx_out.recv().await {
				Ok(v) => v,
				Err(_) => {
					panic!("output channel closed");
				}
			}
		})
	}


	/// Spawn a Future, await it on a worker, and run the resulting system on the main thread.
	///
	/// This is backwards-compatible with older behaviour but implemented on top
	/// of the unified streaming API: the Future is adapted to a single-item
	/// stream and handled by `spawn_for_each_stream`.
	pub fn spawn_and_run<Fut, Out, Marker>(&mut self, fut: Fut)
	where
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + IntoSystem<(), (), Marker>,
	{
		self.spawn_for_each_stream(FutureAsStream::new(fut), |out| out);
	}

	/// Local (non-Send) variant of `spawn_and_run`.
	pub fn spawn_and_run_local<Fut, Out, Marker>(&mut self, fut: Fut)
	where
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + IntoSystem<(), (), Marker>,
	{
		self.spawn_for_each_stream_local(FutureAsStream::new(fut), |out| out);
	}

	/// Spawn a stream on the compute pool and schedule the mapping closure for
	/// each item produced by the stream. Each mapping is converted into a
	/// `CommandQueue` and sent to the main thread via `async_channel`.
	///
	/// - `S` is the stream type produced by the caller.
	/// - `Item` is the item yielded by the stream.
	/// - `map` converts an item into a system (an `IntoSystem`), which will be run
	///   on the main thread, receiving the system params of the original system.
	pub fn spawn_for_each_stream<S, Item, Out, Marker>(
		&mut self,
		stream: S,
		map: impl 'static + Send + Fn(Item) -> Out,
	) where
		S: 'static + Send + Stream<Item = Item>,
		Item: 'static + Send,
		Out: 'static + Send + IntoSystem<(), (), Marker>,
	{
		let (tx, rx) = async_channel::unbounded::<CommandQueue>();
		let task = AsyncComputeTaskPool::get().spawn(async move {
			// Pin the stream so we can `.next().await` it in the loop.
			let mut stream = Box::pin(stream);
			while let Some(item) = stream.next().await {
				let out = map(item);
				let mut queue = CommandQueue::default();
				queue.push(move |world: &mut World| {
					// Non-ZST systems cannot be cached
					world.run_system_once(out).ok();
				});
				// If the receiver has been dropped, stop producing.
				if tx.send(queue).await.is_err() {
					break;
				}
			}
			// Dropping the sender closes the channel and signals completion.
		});
		self.commands.spawn(AsyncStreamTask::new(rx, task));
	}

	/// Local (non-Send) variant of `spawn_for_each_stream`.
	pub fn spawn_for_each_stream_local<S, Item, Out, Marker>(
		&mut self,
		stream: S,
		map: impl 'static + Fn(Item) -> Out,
	) where
		S: 'static + Stream<Item = Item>,
		Item: 'static,
		Out: 'static + Send + IntoSystem<(), (), Marker>,
	{
		let (tx, rx) = async_channel::unbounded::<CommandQueue>();
		let task = AsyncComputeTaskPool::get().spawn_local(async move {
			let mut stream = Box::pin(stream);
			while let Some(item) = stream.next().await {
				let out = map(item);
				let mut queue = CommandQueue::default();
				queue.push(move |world: &mut World| {
					// Non-ZST systems cannot be cached
					world.run_system_once(out).ok();
				});
				if tx.send(queue).await.is_err() {
					break;
				}
			}
		});
		self.commands.spawn(AsyncStreamTask::new(rx, task));
	}
}


/// Immediately Yield n times then finish
pub struct StreamCounter {
	max: usize,
	current: usize,
}

impl StreamCounter {
	pub fn new(count: usize) -> Self {
		Self {
			max: count,
			current: 0,
		}
	}
}

impl Stream for StreamCounter {
	type Item = usize;

	fn poll_next(
		self: Pin<&mut Self>,
		_cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let this = unsafe { self.get_unchecked_mut() };
		if this.current < this.max {
			let v = this.current;
			this.current += 1;
			Poll::Ready(Some(v))
		} else {
			Poll::Ready(None)
		}
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
	pub fn tx(&self) -> async_channel::Sender<CommandQueue> {
		let (tx, rx) = async_channel::unbounded();
		Self { rx, tx }.tx
	}
}


/// A portable channel for sending a [`CommandQueue`] to the world
#[derive(Clone)]
pub struct AsyncQueue {
	tx: async_channel::Sender<CommandQueue>,
}

impl AsyncQueue {
	pub fn new(tx: async_channel::Sender<CommandQueue>) -> Self { Self { tx } }


	pub fn run_system_with_output() {
		todo!("return channel");
	}

	pub fn spawn<B: Bundle>(&self, bundle: B) {
		self.send(move |world: &mut World| {
			world.spawn(bundle);
		});
	}

	pub fn insert_resource<R: Resource>(&self, resource: R) {
		self.send(move |world: &mut World| {
			world.insert_resource(resource);
		});
	}
	pub fn update_resource<R: Resource>(
		&self,
		func: impl FnOnce(Mut<R>) + Send + 'static,
	) {
		self.send(move |world: &mut World| {
			func(world.resource_mut::<R>());
		});
	}

	pub fn send(&self, cmd: impl Command<()>) {
		let mut queue = CommandQueue::default();
		queue.push(cmd);
		self.tx.try_send(queue).expect("Failed to send command. Async queues should be unbounded, was the receiver dropped?");
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
	async fn feels_like_async_but_isnt() {
		let mut app = App::new();
		app.init_resource::<Count>()
			.add_plugins((MinimalPlugins, AsyncPlugin));
		let fut = app
			.world_mut()
			.run_system_cached(|mut commands: AsyncCommands| {
				commands.spawn(async |queue| {
					let next = 1;
					future::yield_now().await;
					queue.update_resource::<Count>(move |mut count| {
						count.0 += next
					});
					future::yield_now().await;
					32
				})
			})
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
