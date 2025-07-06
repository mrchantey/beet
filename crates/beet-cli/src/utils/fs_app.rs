use beet::prelude::*;
use std::num::NonZeroU8;
use std::ops::ControlFlow;
use std::time::Duration;


/// A bevy app runner that will update when a file is added, changed or removed.
///
/// ## Example
///
/// ```rust no_run
/// # use bevy::prelude::*;
/// # use beet::prelude::*;
/// # use beet_cli::prelude::*;
///
/// App::new()
/// 	.run_async(FsApp::default().runner());
///
/// ```
pub struct FsApp {
pub	watcher: FsWatcher,
}


impl Default for FsApp {
	fn default() -> Self {
		// let templates_root_dir = WsPathBuf::default();

		Self {
			watcher: FsWatcher {
				filter: GlobFilter::default()
					.with_exclude("*.git*")
					.with_exclude("*codegen*") // temp until we get fine grained codegen control
					.with_exclude("*target*"),
				// avoid short burst refreshing
				debounce: Duration::from_millis(100),
				..Default::default()
			},
		}
	}
}

impl FsApp {
	/// Update the app whenever a file is created, modified, or deleted.
	// We dont use [`App::set_runner`] because its an async runner and
	// we want to be able to call it from inside a tokio
	pub fn runner(self) -> impl AsyncFnOnce(App) -> AppExit + 'static {
		async |mut app| {
			app.init();

			if let Err(err) =
				Self::on_change(&mut app, WatchEventVec::default())
			{
				eprintln!("Error during initial run: {}", err);
				return AppExit::Error(NonZeroU8::new(1).unwrap());
			}

			let result: Result<AppExit> = async move {
				let mut rx = self.watcher.watch()?;

				while let Some(ev) = rx.recv().await? {
					if ev.has_mutate() {
						match Self::on_change(&mut app, ev)? {
							ControlFlow::Continue(_) => {}
							ControlFlow::Break(exit) => {
								return Ok(exit);
							}
						}
					}
				}
				Ok(AppExit::Success)
			}
			.await;
			result.unwrap_or_else(|err| {
				// non-bevy results havent printed yet
				eprintln!("Error during file change: {}", err);
				AppExit::Error(NonZeroU8::new(1).unwrap())
			})
		}
	}

	/// Updates changed template files
	fn on_change(
		app: &mut App,
		watch_event: WatchEventVec,
	) -> Result<ControlFlow<AppExit>> {
		let start = std::time::Instant::now();
		app.world_mut().send_event_batch(
			watch_event.take().into_iter().filter(|ev| ev.mutated()),
		);
		app.update();
		let elapsed = start.elapsed();
		// TODO per-system profiling https://github.com/bevyengine/bevy/blob/main/docs/profiling.md
		debug!("App updated in {:?}", elapsed);
		match app.should_exit() {
			Some(exit) => Ok(ControlFlow::Break(exit)),
			None => Ok(ControlFlow::Continue(())),
		}
	}
}
