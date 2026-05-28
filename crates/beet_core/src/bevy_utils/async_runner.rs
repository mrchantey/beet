//! Async app runner for Bevy.
//!
//! This module provides the [`AsyncRunner`] which allows running a Bevy [`App`]
//! asynchronously to completion, useful for environments like wasm where
//! `App::run` returns immediately.
//!
//! Two orthogonal concerns compose here: *driving the app* (`app.update()`,
//! which runs the [`BeetAsyncSyncPoint`] driver) and *the task executor* (the
//! pluggable [`AsyncSpawner`]). The runner only drives the app and can itself be
//! `.await`ed inside any host runtime.

use crate::prelude::*;
use core::future::Future;

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
	// wasm: drive our tickable bridge executor (bevy's `spawn_local` uses the
	// untickable JS event loop), so spawned tasks make progress between updates.
	#[cfg(target_arch = "wasm32")]
	super::tick_bridge_executor();
}

/// Yields control to the host executor.
///
/// On native this is a single-poll yield; on wasm we go through `setTimeout(0)`
/// because the JS event loop won't fire pending callbacks (timers, fetches)
/// until we hand control back to it.
async fn yield_to_executor() {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			time_ext::sleep_millis(0).await;
		} else {
			async_ext::yield_now().await;
		}
	}
}

impl AsyncRunner {
	/// Runs an app asynchronously until an [`AppExit`] is triggered.
	pub(crate) async fn run(mut app: App) -> AppExit {
		app.init_plugin::<AsyncPlugin>();
		app.init();

		// outer loop runs when there are no in-flight async tasks
		loop {
			// 1. flush async tasks (also runs update)
			Self::flush_async_tasks(app.world_mut()).await;
			// 2. exit if instructed
			if let Some(exit) = app.should_exit() {
				return exit;
			}
			// 3. yield before the next update
			yield_to_executor().await;
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
			// 1. update first to process the sync point + spawned commands
			world.update_local();
			// 2. tick local tasks in multi-threaded mode
			tick_task_pools();
			// 3. exit if AppExit
			if let Some(exit) = world.should_exit() {
				return Some(exit);
			}
			// 4. exit if no remaining tasks
			if world.resource::<AsyncSpawner>().in_flight() == 0 {
				return None;
			}
			// 5. yield to the executor
			yield_to_executor().await;
		}
	}

	/// Updates the app until `fut` resolves, returning its output.
	///
	/// Ticks task pools after yielding to ensure spawned local tasks make progress.
	/// Runs one final update after the future resolves to process any pending commands.
	pub async fn poll_and_update<F>(
		mut update: impl FnMut(),
		fut: F,
	) -> F::Output
	where
		F: Future,
	{
		let mut fut = Box::pin(fut);
		loop {
			// Update to process the sync point + command queues.
			update();
			// Tick task pools BEFORE polling to ensure newly spawned
			// local tasks are polled in the same tick.
			tick_task_pools();
			if let Some(out) = futures_lite::future::poll_once(&mut fut).await {
				// Run one final update to process any commands the async task
				// produced before completing (e.g. resource modifications).
				update();
				return out;
			}
			// Yield to let the executor poll other tasks.
			yield_to_executor().await;
			// Tick again after yielding to progress any tasks that were
			// waiting on this task to yield.
			tick_task_pools();
		}
	}
}
