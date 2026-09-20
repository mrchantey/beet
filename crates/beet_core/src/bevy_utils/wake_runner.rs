//! [`WakeRunnerPlugin`]: the headless schedule loop whose frame gap is spent
//! running the main thread's task executors rather than sleeping.
use crate::prelude::*;
use bevy::app::PluginsState;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::ComputeTaskPool;
use bevy::tasks::IoTaskPool;
use core::time::Duration;

/// The native headless runner: one [`App::update`] per frame, and between
/// frames the task pools' main-thread executors run until the next frame is
/// due.
///
/// Bevy's `ScheduleRunnerPlugin::run_loop` sleeps out the gap, and a task
/// spawned onto the main thread (every `spawn_local`, and every spawn in a
/// single-threaded build) is only polled by the sync point's tick burst once
/// per update. A future that hops off the thread for a moment, ie any file or
/// store call landing on the `blocking` pool, is woken microseconds later and
/// then waits for the next frame, so each sequential `.await` costs a whole
/// frame however fast its work was. Here the gap is a `block_on` parked on
/// the executors' wakers: a woken task is polled at once, and an idle app
/// still sleeps to the frame.
///
/// A poll that computes for longer than the frame still holds the loop for
/// that long, as it does under any single-threaded executor.
pub struct WakeRunnerPlugin {
	/// The minimum time between updates.
	pub frame: Duration,
}

impl WakeRunnerPlugin {
	/// A runner updating at most every `frame`.
	pub fn new(frame: Duration) -> Self { Self { frame } }

	/// Runs every main-thread executor until `deadline` elapses, parked on
	/// their wakers in between.
	fn run_tasks_for(deadline: Duration) {
		IoTaskPool::get().with_local_executor(|io| {
			AsyncComputeTaskPool::get().with_local_executor(|async_compute| {
				ComputeTaskPool::get().with_local_executor(|compute| {
					let tasks = io.run(
						async_compute
							.run(compute.run(async_ext::yield_forever())),
					);
					futures_lite::future::block_on(futures_lite::future::or(
						tasks,
						time_ext::sleep(deadline),
					));
				})
			})
		});
	}
}

impl Plugin for WakeRunnerPlugin {
	fn build(&self, app: &mut App) {
		let frame = self.frame;
		app.set_runner(move |mut app: App| {
			// as bevy's runner: let the plugins finish before the first update
			if app.plugins_state() != PluginsState::Cleaned {
				while app.plugins_state() == PluginsState::Adding {
					bevy::tasks::tick_global_task_pools_on_main_thread();
				}
				app.finish();
				app.cleanup();
			}
			loop {
				let start = Instant::now();
				app.update();
				if let Some(exit) = app.should_exit() {
					return exit;
				}
				Self::run_tasks_for(frame.saturating_sub(start.elapsed()));
			}
		});
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::app::ScheduleRunnerPlugin;
	use core::sync::atomic::AtomicU32;
	use core::sync::atomic::Ordering;
	use core::time::Duration;
	use std::sync::Arc;

	const FRAME: Duration = Duration::from_millis(30);

	/// Runs `fut` as a task on an app under the wake runner, exits the app
	/// when it ends, and returns the updates run: the app is moved into the
	/// runner, so the count lives outside it.
	fn run(fut: impl 'static + Send + Future<Output = ()>) -> u32 {
		let frames = Arc::new(AtomicU32::new(0));
		let counter = frames.clone();
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins
				.build()
				.disable::<ScheduleRunnerPlugin>()
				.add(WakeRunnerPlugin::new(FRAME)),
			AsyncPlugin::default(),
		))
		.add_systems(Update, move || {
			counter.fetch_add(1, Ordering::SeqCst);
		});
		app.world_mut().run_async(async move |world| {
			fut.await;
			world.write_message(AppExit::Success).await;
		});
		app.run();
		frames.load(Ordering::SeqCst)
	}

	/// Fifty sequential hops off the thread take fifty frames on a sleeping
	/// runner; here they complete inside a few.
	#[crate::test]
	fn polls_a_woken_task_before_the_next_frame() {
		let start = Instant::now();
		let frames = run(async {
			for _ in 0..50 {
				// the async-io reactor thread wakes this one
				time_ext::sleep(Duration::from_millis(1)).await;
			}
		});
		(start.elapsed() < FRAME * 10).xpect_true();
		(frames < 10).xpect_true();
	}

	/// An idle app does not spin: a task sleeping three frames is answered
	/// after about three updates, not thousands.
	#[crate::test]
	fn idles_at_the_frame_rate() {
		let frames = run(async {
			time_ext::sleep(FRAME * 3).await;
		});
		(frames >= 3 && frames <= 6).xpect_true();
	}
}
