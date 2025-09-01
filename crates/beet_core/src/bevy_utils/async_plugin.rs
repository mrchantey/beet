use bevy::ecs::system::SystemParam;
use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::Task;
use bevy::tasks::block_on;
use bevy::tasks::futures_lite::future;


pub struct AsyncPlugin;

impl Plugin for AsyncPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PreUpdate, poll_async_tasks);
	}
}

fn poll_async_tasks(
	mut commands: Commands,
	mut tasks: Query<(Entity, &mut AsyncTask)>,
) {
	for (entity, mut task) in &mut tasks {
		if let Some(mut queue) = block_on(future::poll_once(&mut task.0)) {
			commands.append(&mut queue);
			commands.entity(entity).remove::<AsyncTask>();
		}
	}
}

/// A task returning a [`CommandQueue`] which itsself may spawn
/// another [`AsyncTask`] to be polled in the next frame
#[derive(Component)]
pub struct AsyncTask(Task<CommandQueue>);


#[derive(SystemParam)]
pub struct AsyncCommands<'w, 's> {
	commands: Commands<'w, 's>,
}

impl AsyncCommands<'_, '_> {
	pub fn spawn<Fut>(&mut self, fut: Fut)
	where
		Fut: 'static + Send + Future<Output = CommandQueue>,
		// Out: 'static,
	{
		let task = AsyncComputeTaskPool::get().spawn(fut);
		self.commands.spawn(AsyncTask(task));
	}
	pub fn spawn_and_run<Fut, Out, Marker>(&mut self, fut: Fut)
	where
		Fut: 'static + Send + Future<Output = Out>,
		Out: 'static + Send + IntoSystem<(), (), Marker>,
	{
		let task = AsyncComputeTaskPool::get().spawn(async move {
			let out = fut.await;
			let mut queue = CommandQueue::default();
			queue.push(move |world: &mut World| {
				world.run_system_cached(out).ok();
			});
			// TODO: notify reactive apps that future is ready
			queue
		});
		self.commands.spawn(AsyncTask(task));
	}
	pub fn spawn_and_run_local<Fut, Out, Marker>(&mut self, fut: Fut)
	where
		Fut: 'static + Future<Output = Out>,
		Out: 'static + Send + IntoSystem<(), (), Marker>,
	{
		let task = AsyncComputeTaskPool::get().spawn_local(async move {
			let out = fut.await;
			let mut queue = CommandQueue::default();
			queue.push(move |world: &mut World| {
				world.run_system_cached(out).ok();
			});
			// TODO: notify reactive apps that future is ready
			queue
		});
		self.commands.spawn(AsyncTask(task));
	}
}
