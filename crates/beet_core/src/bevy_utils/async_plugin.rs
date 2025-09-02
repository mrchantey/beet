use async_channel;
use async_channel::TryRecvError;
use bevy::ecs::system::RunSystemOnce;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::Task;
use bevy::tasks::block_on;
use bevy::tasks::futures_lite::Stream;
use bevy::tasks::futures_lite::StreamExt;
use bevy::tasks::futures_lite::future;

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
		app.add_systems(PreUpdate, poll_async_tasks);
	}
}

fn poll_async_tasks(
	mut commands: Commands,
	mut oneshots: Query<(Entity, &mut AsyncTask)>,
	mut streams: Query<(Entity, &mut AsyncStreamTask)>,
) {
	// Existing one-shot handling (keeps backwards compatibility)
	for (entity, mut task) in &mut oneshots {
		if let Some(mut queue) = block_on(future::poll_once(&mut task.0)) {
			commands.append(&mut queue);
			commands.entity(entity).remove::<AsyncTask>();
		}
	}

	// Streaming handling: drain all ready command queues produced by streams
	for (entity, stream_task) in &mut streams {
		loop {
			match stream_task.receiver.try_recv() {
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

/// Backwards-compatible one-shot task returning a `CommandQueue`.
#[derive(Component)]
pub struct AsyncTask(Task<CommandQueue>);

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
}

impl AsyncCommands<'_, '_> {
	/// Spawn a background future which returns a `CommandQueue`.
	///
	/// This is a low-level primitive used by the system. The future runs on the
	/// compute thread pool and may produce a `CommandQueue` to be applied later.
	pub fn spawn<Fut>(&mut self, fut: Fut)
	where
		Fut: 'static + Send + Future<Output = CommandQueue>,
	{
		let task = AsyncComputeTaskPool::get().spawn(fut);
		self.commands.spawn(AsyncTask(task));
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::as_beet::*;
	use beet_core_macros::async_system;
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

	#[derive(Resource)]
	struct Count(usize);

	#[async_system]
	async fn exclusive_async_system(world: &mut World) {
		let _ = future::yield_now().await;
	}

	#[test]
	fn futures() {
		let mut app = App::new();
		app.insert_resource(Count(0))
			.add_plugins((MinimalPlugins, AsyncPlugin));

		#[async_system]
		async fn my_system(mut count: ResMut<Count>) {
			let _ = future::yield_now().await;
			assert_eq!(count.0, 0);
			count.0 += 1;
			let _ = future::yield_now().await;
			{
				let _ = future::yield_now().await;
			}
			assert_eq!(count.0, 1);
			count.0 += 1;
			let _ = future::yield_now().await;
			assert_eq!(count.0, 2);
			count.0 += 1;
		}

		app.world_mut().run_system_cached(my_system).ok();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(1);
		app.update();
		app.update();
		app.update();
		app.update();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(0);
		app.world_mut().resource::<Count>().0.xpect().to_be(3);
	}
	#[test]
	fn observers() {
		let mut app = App::new();
		app.insert_resource(Count(0))
			.add_plugins((MinimalPlugins, AsyncPlugin));

		#[derive(Event)]
		struct MyEvent;
		// compiles
		#[async_system]
		async fn my_exclusive_observer(_: Trigger<MyEvent>, world: &mut World) {
			let _ = future::yield_now().await;
			assert_eq!(world.resource::<Count>().0, 0);
			world.resource_mut::<Count>().0 += 1;
			let _ = future::yield_now().await;
			{
				let _ = future::yield_now().await;
			}
			assert_eq!(world.resource::<Count>().0, 1);
			world.resource_mut::<Count>().0 += 1;
			let _ = future::yield_now().await;
			assert_eq!(world.resource::<Count>().0, 2);
			world.resource_mut::<Count>().0 += 1;
		}

		#[async_system]
		async fn my_observer(_: Trigger<MyEvent>, mut count: ResMut<Count>) {
			let _ = future::yield_now().await;
			assert_eq!(count.0, 0);
			count.0 += 1;
			let _ = future::yield_now().await;
			{
				let _ = future::yield_now().await;
			}
			assert_eq!(count.0, 1);
			count.0 += 1;
			let _ = future::yield_now().await;
			assert_eq!(count.0, 2);
			count.0 += 1;
		}
		app.world_mut().add_observer(my_observer).trigger(MyEvent);
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(1);
		app.update();
		app.update();
		app.update();
		app.update();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(0);
		app.world_mut().resource::<Count>().0.xpect().to_be(3);
	}
	#[test]
	fn streams() {
		let mut app = App::new();
		app.insert_resource(Count(0))
			.add_plugins((MinimalPlugins, AsyncPlugin));

		#[async_system]
		async fn my_system(mut count: ResMut<Count>) {
			let mut stream = StreamCounter::new(3);
			while let index = stream.next().await {
				{
					let _ = future::yield_now().await;
				}
				assert_eq!(index, count.0);
				count.0 += 1;
			}
		}

		app.world_mut().run_system_cached(my_system).ok();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(1);
		app.update();
		app.update();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(0);
		app.world_mut().resource::<Count>().0.xpect().to_be(3);
	}

	#[sweet::test]
	async fn returns_value_future() {
		#[async_system]
		async fn my_system(mut count: ResMut<Count>) -> usize {
			let _ = future::yield_now().await;
			let before = count.0;
			count.0 += 5;
			if count.0 == 1 {
				let _ = future::yield_now().await;
				return count.0;
			}
			let _ = future::yield_now().await;
			// return before + count.0;
			before + count.0
		}

		let mut app = App::new();
		app.insert_resource(Count(10))
			.add_plugins((MinimalPlugins, AsyncPlugin));

		let fut = app.world_mut().run_system_cached(my_system).unwrap();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(1);

		// Progress async work to completion
		app.update();
		app.update();

		fut.await.xpect().to_be(25);
		// After completion, the stream task should be removed
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(0);
	}

	#[sweet::test]
	async fn complex() {
		/// an async system using futures and streams to count to five
		#[async_system]
		async fn my_system(mut count: ResMut<Count>) -> usize {
			future::yield_now().await;
			count.0 += 1;
			assert_eq!(count.0, 1);
			while let index = StreamCounter::new(4).await {
				assert_eq!(count.0, index + 1);
				count.0 += 1;
			}
			assert_eq!(count.0, 5);
			count.0
		}
		let mut app = App::new();
		app.insert_resource(Count(0))
			.add_plugins((MinimalPlugins, AsyncPlugin));

		let fut = app.world_mut().run_system_cached(my_system).unwrap();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(1);

		// Progress async work to completion
		app.update();
		app.update();

		fut.await.xpect().to_be(5);
		// After completion, the stream task should be removed
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(0);
	}
	#[sweet::test]
	async fn results() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin));

		#[async_system]
		async fn my_system() -> Result {
			let _ = Ok(())?;
			let _ = future::yield_now().await;
			// let _ = async move { Ok(()) }.await?;
			let _ = future::yield_now().await;
			{
				let _ = Err("foobar".into())?;
				let _ = future::yield_now().await;
			}
			let _ = future::yield_now().await;
			let _ = Ok(())?;
			Ok(())
		}

		let fut = app.world_mut().run_system_cached(my_system).unwrap();
		app.update();
		app.update();
		app.world_mut()
			.query_once::<&AsyncStreamTask>()
			.iter()
			.count()
			.xpect()
			.to_be(0);
		fut.await.unwrap_err().to_string().xpect().to_be("foobar\n");
	}
}
