use crate::prelude::*;
use bevy::prelude::*;

pub struct AsyncRunner;


#[extend::ext]
pub impl App {
	/// A non-send runner
	fn run_async(&mut self) -> impl 'static + Future<Output = AppExit> {
		AsyncRunner::run(std::mem::take(self))
	}
}


impl AsyncRunner {
	/// Uses the [`AsyncChannel::rx`] as a signal to run updates,
	/// this means that the rx in poll_async_tasks should be empty during updates
	/// Any triggered [`AppExit`] will cause an exit after the current flush completes.
	pub async fn run(mut app: App) -> AppExit {
		app.init_plugin(AsyncPlugin);
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
			// println!("no async tasks in flight, sleeping..");
			time_ext::sleep_millis(100).await;
		}
	}
	/// Run an loop at regular updates until all tasks have completed or
	/// an AppExit is triggered.
	pub async fn flush_async_tasks(world: &mut World) -> Option<AppExit> {
		let mut task_query = world.query::<&mut AsyncTask>();
		let rx = world.resource::<AsyncChannel>().rx.clone();
		// tried an exponential backoff here, made streaming responses
		// ie from agents extremely slow, i guess reqwest etc requires regular polling
		// to receive bytes?
		loop {
			// 1. update
			world.update();

			// 2. flush rx
			while let Ok(mut queue) = rx.try_recv() {
				world.commands().append(&mut queue);
			}

			// 3. exit if AppExit
			if let Some(exit) = world.should_exit() {
				return Some(exit);
			}

			// 4. exit if no remaining tasks
			if task_query.query(world).is_empty() {
				return None;
			}

			// 5. short delay
			time_ext::sleep_millis(1).await;
			// }
		}
	}
}
