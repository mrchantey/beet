use crate::prelude::*;
use beet_core::prelude::*;
use clap::Parser;
use dotenv::dotenv;
use std::time::Duration;


/// Entrypoint for a beet launch sequence.
/// All environment variables in a `.env` file in the current directory will be loaded.
#[derive(Debug, Default, Clone, Parser)]
pub struct LaunchRunner {
	#[arg(short, long)]
	pub watch: bool,
	#[clap(flatten)]
	pub(crate) config_overrides: ConfigOverrides,
	#[clap(flatten)]
	pub(crate) lambda_config: LambdaConfig,
	#[arg(short, long)]
	pub package: Option<String>,
	// /// 🦀 the commands that will be used to build the binary 🦀
	// #[clap(flatten)]
	// pub(crate) build_cmd: CargoBuildCmd,
}

impl LaunchRunner {
	pub fn runner(app: App) -> AppExit { Self::parse().run(app) }
	pub fn run(self, mut app: App) -> AppExit {
		dotenv().ok();
		PrettyTracing::default().init();
		app.add_plugins(self.config_overrides);
		let mut build_cmd = CargoBuildCmd::default();
		build_cmd.package = self.package;
		app.insert_resource(build_cmd);
		app.insert_resource(self.lambda_config);

		let result = match self.watch {
			true => Self::watch(app),
			false => app.run_once(),
		};
		result
	}

	/// Run in watch mode, running again if any file
	/// specified in the [`WorkspaceConfig`] changes.
	#[tokio::main]
	async fn watch(mut app: App) -> AppExit {
		let config = app
			.init_resource::<WorkspaceConfig>()
			.world()
			.resource::<WorkspaceConfig>();
		let cwd = config.root_dir.into_abs();
		let filter = config.snippet_filter.clone();

		app.add_plugins((MinimalPlugins, FsWatcherPlugin {
			watcher: FsWatcher {
				cwd,
				filter,
				debounce: Duration::from_millis(100),
			},
		}))
		.run_async()
		.await
	}
}
