use std::time::Duration;

use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use clap::Parser;





#[derive(Debug, Default, Clone, Parser)]
pub struct LaunchRunner {
	#[arg(short, long)]
	pub watch: bool,
}


impl LaunchRunner {
	pub fn runner(app: App) -> AppExit { Self::parse().run(app) }
	pub fn run(self, mut app: App) -> AppExit {
		let result = match self.watch {
			true => self.watch(app),
			false => app.run_once(),
		};
		result
	}
	#[tokio::main]
	async fn watch(self, mut app: App) -> AppExit {
		let config = app
			.init_resource::<WorkspaceConfig>()
			.world()
			.resource::<WorkspaceConfig>();
		let cwd = config.root_dir.into_abs();
		let filter = config.filter.clone();

		app.run_async(
			FsApp {
				watcher: FsWatcher {
					cwd: cwd.0,
					filter,
					debounce: Duration::from_millis(100),
				},
			}
			.runner(),
		)
		.await
	}
}
