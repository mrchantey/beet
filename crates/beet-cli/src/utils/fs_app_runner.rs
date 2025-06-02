use anyhow::Result;
use bevy::prelude::*;
use std::num::NonZeroU8;
use std::time::Duration;
use sweet::prelude::*;


/// An alternative to the default app runner [`App::set_runner`]
/// 
/// ## Example
/// 
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_cli::prelude::*;
/// 
/// let runner = FsAppRunner::default();
///
/// App::new()
/// 	.set_runner(runner.into_app_runner())
/// 	.run();
/// 
/// ```
pub struct FsAppRunner {
	watcher: FsWatcher,
}


impl Default for FsAppRunner {
	fn default() -> Self {
		let templates_root_dir = WorkspacePathBuf::default();

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

impl FsAppRunner {
	/// Update the app whenever a file is created, modified, or deleted.
	pub fn into_app_runner(self) -> impl FnOnce(App) -> AppExit + 'static {
		|mut app| {
			if let Err(err) =
				Self::on_change(&mut app, WatchEventVec::default())
			{
				eprintln!("Error during initial run: {}", err);
				return AppExit::Error(NonZeroU8::new(1).unwrap());
			}

			let body = async move {
				let watcher_result = self
					.watcher
					.watch_async(move |ev| {
						if !ev.has_mutate() {
							return Ok(());
						}
						Self::on_change(&mut app, ev)?;
						Ok(())
					})
					.await;
				match watcher_result {
					Ok(_) => AppExit::Success,
					Err(err) => {
						eprintln!("Error during file change: {}", err);
						AppExit::Error(NonZeroU8::new(1).unwrap())
					}
				}
			};
			tokio::runtime::Builder::new_multi_thread()
				.enable_all()
				.build()
				.expect("Failed building the Runtime")
				.block_on(body)
		}
	}

	fn on_change(app: &mut App, watch_event: WatchEventVec) -> Result<()> {
		println!("In main loop");
		// TODO insert events
		app.update();
		match app.should_exit() {
			Some(AppExit::Success) => return Ok(()),
			Some(AppExit::Error(err)) => {
				Err(anyhow::anyhow!("App exited with error: {}", err))?;
			}
			None => {}
		}
		Ok(())
	}
}
