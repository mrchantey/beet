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
	pub beet_build_args: BeetBuildArgs,
}

impl RunBuild {
	/// Run once
	pub async fn build(self) -> Result {
		App::new()
			.insert_resource(self.build_cmd)
			.add_plugins(self.beet_build_args)
			.run_once()
			.into_result()
	}


	/// Run in watch mode with a file watcher
	pub async fn run(self) -> Result {
		App::new()
			.insert_resource(self.build_cmd)
			.add_plugins(self.beet_build_args)
			.run_async(FsApp::default().runner())
			.await
			.into_result()
	}
}
