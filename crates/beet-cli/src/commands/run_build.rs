use crate::prelude::*;
use beet::prelude::*;
use clap::Parser;

/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct RunBuild {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	pub build_cmd: CargoBuildCmd,
	/// Determine the config location and which builds steps to run
	#[command(flatten)]
	pub load_beet_config: LoadBeetConfig,
}

impl RunBuild {
	pub async fn run(self) -> anyhow::Result<()> {
		use bevy::ecs::error::GLOBAL_ERROR_HANDLER;
		GLOBAL_ERROR_HANDLER
			.set(bevy::ecs::error::panic)
			.expect("The error handler can only be set once, globally.");

		// specifying 'only' means just run once
		let run_once = !self.load_beet_config.only.is_empty();

		let mut app = App::new();
		app.insert_resource(self.build_cmd.clone()).add_plugins((
			self.load_beet_config.clone(),
			NodeTokensPlugin::default(),
			StaticScenePlugin::default(),
			RouteCodegenPlugin::default(),
			ClientIslandCodegenPlugin::default(),
		));

		if run_once {
			app.run_once().anyhow()
		} else {
			// .set_error_handler(warn)
			app.run_async(FsApp::default().runner()).await.anyhow()
		}
	}
}
