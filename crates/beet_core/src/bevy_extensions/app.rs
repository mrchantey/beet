//! Extension methods for Bevy's [`App`].

use crate::prelude::*;
use bevy::app::Plugins;
use bevy::app::PluginsState;
use bevy::prelude::*;
#[cfg(feature = "std")]
use bevy::tasks::IoTaskPool;
#[cfg(feature = "std")]
use bevy::tasks::Task;

/// The runners and builders on [`App`]: what it registers is a plugin variant
/// instead, see [`TypePlugin`] and [`OwnedPlugin`].
#[extend::ext(name=BeetCoreAppExt)]
#[allow(async_fn_in_trait)]
pub impl App {
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
	#[cfg(all(feature = "bevy_async", feature = "std"))]
	fn run_async(&mut self) -> impl 'static + Future<Output = AppExit> {
		AsyncRunner::run(core::mem::take(self))
	}

	/// Drives the app until it settles, ticking the task pools between updates.
	/// Unlike [`run_async`](Self::run_async) it returns once the work settles
	/// rather than looping to [`AppExit`].
	///
	/// Use in async contexts (eg tests) to await multi-frame async convergence,
	/// such as a bridged `spawn_local` task writing a field then a reactive
	/// rebuild, without hand-rolling the update + tick loop. See
	/// [`AsyncRunner::settle_async_tasks`] for the settle semantics.
	#[cfg(all(feature = "bevy_async", feature = "std"))]
	async fn update_async(&mut self) {
		AsyncRunner::settle_async_tasks(self.world_mut()).await;
	}

	/// Runs an IO task to completion, polling at 10 millisecond intervals.
	#[cfg(feature = "std")]
	async fn run_io_task<F, O>(&mut self, fut: F) -> O
	where
		F: Future<Output = O> + 'static + Send,
		O: 'static + Send,
	{
		self.await_io_task(IoTaskPool::get().spawn(fut)).await
	}
	/// Runs a local IO task to completion, polling at 10 millisecond intervals.
	#[cfg(feature = "std")]
	async fn run_io_task_local<F, O>(&mut self, fut: F) -> O
	where
		F: Future<Output = O> + 'static,
		O: 'static,
	{
		self.await_io_task(IoTaskPool::get().spawn_local(fut)).await
	}
	/// Awaits an IO task, updating the app while waiting.
	#[cfg(feature = "std")]
	async fn await_io_task<O>(&mut self, task: Task<O>) -> O {
		self.add_type::<TaskPoolPlugin>();
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

	/// Waits for every plugin to be ready, then finishes and cleans them up,
	/// so the app is fully built before its first update.
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
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[derive(Default, Resource)]
	struct Foo(Vec<f32>);

	#[crate::test]
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
