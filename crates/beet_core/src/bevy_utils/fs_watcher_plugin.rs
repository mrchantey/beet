use crate::prelude::*;
use std::time::Duration;


/// Adds an async task that emits a [`WatchEvent`] when files change
///
/// ## Example
///
/// ```rust no_run
/// # use beet_core::prelude::*;
/// # async fn run(){
/// App::new()
///   .add_plugins((MinimalPlugins, FsWatcherPlugin::default()))
///   .run_async().await;
/// # }
/// ```
pub struct FsWatcherPlugin {
	pub watcher: FsWatcher,
}

impl Plugin for FsWatcherPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AsyncPlugin)
			.add_message::<WatchEvent>()
			.insert_resource(self.watcher.clone())
			.add_systems(PreStartup, watch_file_changes);
	}
}

fn watch_file_changes(watcher: Res<FsWatcher>, mut commands: AsyncCommands) {
	let watcher = watcher.clone();
	commands.run(async move |queue| {
		let mut rx = watcher.watch()?;
		while let Some(ev) = rx.recv().await? {
			if ev.has_mutate() {
				let mutated = ev.take().into_iter().filter(|ev| ev.mutated());
				queue.write_message_batch(mutated);
			}
		}

		Ok(())
	});
}

impl Default for FsWatcherPlugin {
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

#[cfg(test)]
#[cfg(not(feature = "multi_threaded"))]
mod test {
	use super::*;
	use bevy::ecs::schedule::common_conditions;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		let touched = Store::default();
		app.add_plugins((MinimalPlugins, FsWatcherPlugin {
			watcher: FsWatcher {
				cwd: AbsPathBuf::new_workspace_rel("").unwrap(),
				filter: GlobFilter::default(),
				..default()
			},
		}))
		.add_systems(
			Update,
			(move |mut reader: MessageReader<WatchEvent>,
			       mut commands: Commands| {
				for ev in reader.read() {
					touched.push(ev.path.clone());
				}
				commands.write_message(AppExit::Success);
			})
			.run_if(common_conditions::on_message::<WatchEvent>),
		);

		app.init();
		app.update();

		fs_ext::write(
			AbsPathBuf::new_workspace_rel(
				"target/tests/beet_core/fs_app/file.txt",
			)
			.unwrap(),
			"foobar",
		)
		.unwrap();


		app.run_async().await;

		touched.get().xpect_any(|item| {
			// might be the directory created or the file created
			item.to_string_lossy().contains("target/tests")
			// item.to_string_lossy().contains("fs_app/file.txt")
		});
	}
}
