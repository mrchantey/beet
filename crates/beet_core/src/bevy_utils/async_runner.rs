//! Async app runner for Bevy.
//!
//! This module provides the [`AsyncRunner`] which allows running a Bevy [`App`]
//! asynchronously to completion, useful for environments like wasm where
//! `App::run` returns immediately.

use crate::prelude::*;
use async_channel::Receiver;
use async_channel::TryRecvError;

/// Runner for executing Bevy apps asynchronously.
///
/// This is particularly useful in wasm environments where `App::run`
/// returns immediately without actually running the app.
pub struct AsyncRunner;

/// Ticks global task pools to progress local tasks.
///
/// This is required because `spawn_local` tasks can only be polled by the
/// thread that owns the LocalExecutor.
#[inline]
fn tick_task_pools() {
	#[cfg(not(target_arch = "wasm32"))]
	bevy::tasks::tick_global_task_pools_on_main_thread();
}

impl AsyncRunner {
	/// Runs an app asynchronously until an [`AppExit`] is triggered.
	pub(crate) async fn run(mut app: App) -> AppExit {
		app.init_plugin::<AsyncPlugin>();
		app.init();

		// this is an outer loop that will run when there are no
		// in-flight async tasks. We'll just do a 100ms update loop
		loop {
			// 1. flush async tasks (also runs update)
			Self::flush_async_tasks(app.world_mut()).await;
			// 2. exit if instructed
			if let Some(exit) = app.should_exit() {
				return exit;
			}
			// 3. delay next update
			// TODO no idea how long the correct duration is here,
			// does it depend on use-case?
			time_ext::sleep_millis(1).await;
		}
	}

	/// Runs an update loop until all tasks have completed or an AppExit is triggered.
	///
	/// Note that some tasks like http/socket listeners will never complete,
	/// in which case this will never return.
	async fn flush_async_tasks(world: &mut World) -> Option<AppExit> {
		// yield required for wasm to spawn tasks
		async_ext::yield_now().await;

		loop {
			// 1. update first to process command queues
			world.update_local();
			// 2. tick local tasks in multi-threaded mode
			tick_task_pools();
			// 3. exit if AppExit
			if let Some(exit) = world.should_exit() {
				return Some(exit);
			}
			// 4. exit if no remaining tasks
			if world.resource::<AsyncChannel>().task_count() == 0 {
				return None;
			}
			// 5. short delay
			time_ext::sleep_millis(1).await;
		}
	}

	/// Updates the world in 1ms increments until recv has a value.
	///
	/// Ticks task pools after yielding to ensure spawned local tasks make progress.
	/// Runs one final update after receiving the result to process any pending commands.
	pub async fn poll_and_update<T>(
		mut update: impl FnMut(),
		recv: Receiver<T>,
	) -> T {
		loop {
			match recv.try_recv() {
				Ok(out) => {
					// Run one final update to process any commands the async task
					// sent before completing (e.g. resource modifications)
					update();
					return out;
				}
				Err(TryRecvError::Empty) => {
					// Update to process command queues
					update();
					// Tick task pools BEFORE yielding to ensure newly spawned
					// local tasks are polled in the same tick
					tick_task_pools();
					// Yield to let the executor poll other tasks.
					// On WASM we need to actually sleep to return control to
					// the JS event loop, otherwise setTimeout callbacks never fire.
					#[cfg(target_arch = "wasm32")]
					time_ext::sleep_millis(1).await;
					#[cfg(not(target_arch = "wasm32"))]
					async_ext::yield_now().await;
					// Tick again after yielding to progress any tasks that were
					// waiting on this task to yield
					tick_task_pools();
				}
				Err(TryRecvError::Closed) => {
					unreachable!("we control the send");
				}
			}
		}
	}
}
