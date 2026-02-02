//! Extension methods for Bevy's [`App`].

use crate::prelude::*;
use bevy::app::MainScheduleOrder;
use bevy::app::Plugins;
use bevy::app::PluginsState;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use bevy::tasks::Task;
use std::time::Duration;

/// Extension trait adding utility methods to [`App`].
#[extend::ext(name=BeetCoreAppExt)]
#[allow(async_fn_in_trait)]
pub impl App {
	/// Adds the plugin if it hasn't been added yet.
	fn init_plugin<T: Plugin + Default>(&mut self) -> &mut Self {
		if self.get_added_plugins::<T>().is_empty() {
			self.add_plugins(T::default());
		}
		self
	}
	/// Adds the plugin if it hasn't been added yet.
	fn init_plugin_with<T: Plugin>(&mut self, plugin: T) -> &mut Self {
		if self.get_added_plugins::<T>().is_empty() {
			self.add_plugins(plugin);
		}
		self
	}
	/// Spawns an entity with the given bundle, returning self for chaining.
	fn spawn_then(&mut self, bundle: impl Bundle) -> &mut Self {
		self.world_mut().spawn(bundle);
		self
	}

	/// Sets the error handler if one hasn't been set yet,
	/// otherwise does nothing.
	fn try_set_error_handler(
		&mut self,
		handler: bevy::ecs::error::ErrorHandler,
	) -> &mut Self {
		if self.get_error_handler().is_none() {
			self.set_error_handler(handler);
		}
		self
	}

	/// Registers a schedule in the main schedule order before the specified schedule.
	///
	/// # Panics
	///
	/// Panics if the other schedule has not been registered yet.
	fn insert_schedule_before(
		&mut self,
		before: impl ScheduleLabel,
		schedule: impl Clone + ScheduleLabel,
	) -> &mut Self {
		self.init_schedule(schedule.clone());
		let mut main_schedule_order =
			self.world_mut().resource_mut::<MainScheduleOrder>();
		main_schedule_order.insert_before(before, schedule);
		self
	}
	/// Registers a schedule in the main schedule order after the specified schedule.
	///
	/// # Panics
	///
	/// Panics if the other schedule has not been registered yet.
	fn insert_schedule_after(
		&mut self,
		after: impl ScheduleLabel,
		schedule: impl Clone + ScheduleLabel,
	) -> &mut Self {
		self.init_schedule(schedule.clone());
		let mut main_schedule_order =
			self.world_mut().resource_mut::<MainScheduleOrder>();
		main_schedule_order.insert_after(after, schedule);
		self
	}

	/// Initializes and runs a single update, then returns the exit status.
	fn run_once(&mut self) -> Option<AppExit> {
		self.init();
		self.update();
		self.should_exit()
	}

	/// Runs the app in a loop without breaks, useful for nested apps in wasm.
	fn run_loop(&mut self) -> AppExit {
		self.init();
		loop {
			self.update();
			if let Some(exit_code) = self.should_exit() {
				return exit_code;
			}
		}
	}

	/// Runs the app asynchronously, useful for wasm where `App::run` returns immediately.
	fn run_async(&mut self) -> impl 'static + Future<Output = AppExit> {
		AsyncRunner::run(std::mem::take(self))
	}

	/// Runs an IO task to completion, polling at 10 millisecond intervals.
	async fn run_io_task<F, O>(&mut self, fut: F) -> O
	where
		F: Future<Output = O> + 'static + Send,
		O: 'static + Send,
	{
		self.await_io_task(IoTaskPool::get().spawn(fut)).await
	}
	/// Runs a local IO task to completion, polling at 10 millisecond intervals.
	async fn run_io_task_local<F, O>(&mut self, fut: F) -> O
	where
		F: Future<Output = O> + 'static,
		O: 'static,
	{
		self.await_io_task(IoTaskPool::get().spawn_local(fut)).await
	}
	/// Awaits an IO task, updating the app while waiting.
	async fn await_io_task<O>(&mut self, task: Task<O>) -> O {
		self.init_plugin::<TaskPoolPlugin>();
		// spin up async task pool
		self.run_once();

		while !task.is_finished() {
			self.update();
			crate::time_ext::sleep_millis(10).await;
		}
		// only await task when its ready, app must update
		// to poll futures
		task.await
	}

	/// Ensures the app is fully initialized before updates.
	// from bevy_app https://github.com/bevyengine/bevy/blob/a1f4e56610c090b44f8b4a8f3eb56aeda5eb9669/crates/bevy_app/src/app.rs#L1392
	fn init(&mut self) -> &mut Self {
		while self.plugins_state() == PluginsState::Adding {
			#[cfg(not(target_arch = "wasm32"))]
			bevy::tasks::tick_global_task_pools_on_main_thread();
		}
		self.finish();
		self.cleanup();
		self
	}

	/// Adds a non-send plugin to the app.
	fn add_non_send_plugin(&mut self, plugin: impl NonSendPlugin) -> &mut Self {
		plugin.build(self);
		self
	}

	/// Spawns an entity with the given bundle.
	fn spawn(&mut self, bundle: impl Bundle) -> &mut Self {
		self.world_mut().spawn(bundle);
		self
	}

	/// Advances time by the given duration then updates.
	///
	/// # Warning
	///
	/// Using this with [`MinimalPlugins`] or other time management systems
	/// will produce unexpected results.
	fn update_with_duration(&mut self, duration: Duration) -> &mut Self {
		self.world_mut().resource_mut::<Time>().advance_by(duration);
		self.update();
		// reset the delta etc
		self.world_mut()
			.resource_mut::<Time>()
			.advance_by(Duration::ZERO);
		self
	}
	/// Advances time by the given seconds then updates.
	///
	/// # Warning
	///
	/// Using this with [`MinimalPlugins`] or other time management systems
	/// will produce unexpected results.
	fn update_with_secs(&mut self, secs: u64) -> &mut Self {
		self.update_with_duration(Duration::from_secs(secs))
	}
	/// Advances time by the given milliseconds then updates.
	///
	/// # Warning
	///
	/// Using this with [`MinimalPlugins`] or other time management systems
	/// will produce unexpected results.
	fn update_with_millis(&mut self, millis: u64) -> &mut Self {
		self.update_with_duration(Duration::from_millis(millis))
	}
	/// Calls `update` and returns `self` for method chaining.
	fn update_then(&mut self) -> &mut Self {
		self.update();
		self
	}

	/// Adds plugins and returns `self` for method chaining.
	#[track_caller]
	fn with_plugins<M>(mut self, plugins: impl Plugins<M>) -> Self {
		self.add_plugins(plugins);
		self
	}

	/// Spawns an entity and returns `self` for method chaining.
	#[track_caller]
	fn with_entity<M>(mut self, bundle: impl Bundle) -> Self {
		self.world_mut().spawn(bundle);
		self
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[derive(Default, Resource)]
	struct Foo(Vec<f32>);

	#[test]
	fn time() {
		let mut app = App::new();
		app.init_resource::<Foo>()
			.init_resource::<Time>()
			.add_systems(Update, |time: Res<Time>, mut foo: ResMut<Foo>| {
				foo.0.push(time.delta_secs());
			});
		app.update();
		app.update_with_millis(10);
		app.world_mut()
			.resource::<Time>()
			.delta_secs()
			.xpect_eq(0.0);
		app.update_with_secs(10);
		app.update();
		app.world_mut()
			.resource::<Foo>()
			.0
			.xpect_eq(vec![0., 0.01, 10., 0.]);
	}
}
