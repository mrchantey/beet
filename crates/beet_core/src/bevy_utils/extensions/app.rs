use crate::prelude::NonSendPlugin;
use bevy::app::MainScheduleOrder;
use bevy::app::PluginsState;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[extend::ext(name=BeetCoreAppExt)]
#[allow(async_fn_in_trait)]
pub impl App {
	/// Add a plugin to the app, if it hasn't been added yet.
	fn init_plugin<T: Plugin>(&mut self, plugin: T) -> &mut Self {
		if self.get_added_plugins::<T>().is_empty() {
			self.add_plugins(plugin);
		}
		self
	}



	/// Register this schedule in the main schedule order after the specified schedule
	/// # Panics
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
	/// Register this schedule in the main schedule order after the specified schedule
	/// # Panics
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

	fn run_once(&mut self) -> AppExit {
		self.init();
		self.update();
		self.should_exit().unwrap_or(AppExit::Success)
	}
	/// run an io task to completion, polling at 10 millisecond intervals
	#[cfg(not(target_arch = "wasm32"))]
	// task::is_finished not found in wasm???
	async fn run_io_task<F, O>(&mut self, fut: F) -> O
	where
		F: Future<Output = O> + 'static,
		O: 'static,
	{
		use bevy::tasks::IoTaskPool;

		self.init_plugin(TaskPoolPlugin::default());
		// spin up async task pool
		self.run_once();

		let task = IoTaskPool::get().spawn(fut);
		// is_finished not found in wasm???
		while !task.is_finished() {
			self.update();
			crate::time_ext::sleep_millis(10).await;
		}
		// only await task when its ready, app must update
		// to poll futures
		task.await
	}

	/// Call this on custom runners before update
	/// to ensure that the app is fully initialized.
	// from bevy_app https://github.com/mrchantey/bevy/blob/a1f4e56610c090b44f8b4a8f3eb56aeda5eb9669/crates/bevy_app/src/app.rs#L1392
	fn init(&mut self) -> &mut Self {
		while self.plugins_state() == PluginsState::Adding {
			#[cfg(not(target_arch = "wasm32"))]
			bevy::tasks::tick_global_task_pools_on_main_thread();
		}
		self.finish();
		self.cleanup();
		self
	}

	fn add_non_send_plugin(&mut self, plugin: impl NonSendPlugin) -> &mut Self {
		plugin.build(self);
		self
	}
}
