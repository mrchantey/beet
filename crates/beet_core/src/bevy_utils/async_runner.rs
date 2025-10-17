use crate::prelude::*;
use async_channel::Receiver;
use async_channel::TryRecvError;

pub struct AsyncRunner;


#[extend::ext]
pub impl App {
	/// A non-send runner
	fn run_async(&mut self) -> impl 'static + Future<Output = AppExit> {
		AsyncRunner::run(std::mem::take(self))
	}
}


impl AsyncRunner {
	pub async fn run(mut app: App) -> AppExit {
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
			// no idea how long to sleep for
			// println!("no async tasks in flight, sleeping..");
			time_ext::sleep_millis(1).await;
		}
	}

	/// Run an loop at regular updates until all tasks have completed or
	/// an AppExit is triggered. Note that some tasks like http/socket listeners
	/// will never complete in which case this will never return.
	pub async fn flush_async_tasks(world: &mut World) -> Option<AppExit> {
		// yield required for wasm to spawn tasks
		async_ext::yield_now().await;

		loop {
			// 1. update
			world.update();
			// 2. exit if AppExit
			if let Some(exit) = world.should_exit() {
				return Some(exit);
			}
			// 3. exit if no remaining tasks
			if world.resource::<AsyncChannel>().task_count() == 0 {
				return None;
			}
			// 4. short delay
			time_ext::sleep_millis(1).await;
		}
	}
	/// update the world in 1ms increments until recv has a value
	pub async fn poll_and_update<T>(
		mut update: impl FnMut(),
		recv: Receiver<T>,
	) -> T {
		loop {
			match recv.try_recv() {
				Ok(out) => return out,
				Err(TryRecvError::Empty) => {
					update();
					time_ext::sleep_millis(1).await;
				}
				Err(TryRecvError::Closed) => {
					unreachable!("we control the send");
				}
			}
		}
	}
}
