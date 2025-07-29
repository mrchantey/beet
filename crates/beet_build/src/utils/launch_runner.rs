use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use std::str::FromStr;
use std::time::Duration;





#[derive(Debug, Default, Clone, Parser)]
pub struct LaunchRunner {
	#[arg(short, long)]
	pub watch: bool,
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	pub(crate) build_cmd: CargoBuildCmd,
	/// Only execute the provided build steps,
	/// options are "routes", "snippets", "client-islands", "compile-server", "export-ssg", "compile-wasm", "run-server"
	#[arg(long, value_delimiter = ',', value_parser = parse_flags)]
	pub(crate) only: Vec<BuildFlag>,
}

fn parse_flags(s: &str) -> Result<BuildFlag, String> { BuildFlag::from_str(s) }


impl LaunchRunner {
	pub fn runner(app: App) -> AppExit { Self::parse().run(app) }
	pub fn run(self, mut app: App) -> AppExit {
		init_pretty_tracing(bevy::log::Level::DEBUG);

		if !self.only.is_empty() {
			app.insert_resource(BuildFlags::Only(self.only.clone()));
		}
		app.insert_resource(self.build_cmd.clone());

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
