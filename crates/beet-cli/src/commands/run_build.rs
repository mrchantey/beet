use crate::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;
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
		App::new()
			.insert_resource(self.build_cmd.clone())
			.add_plugins((
				self.load_beet_config.clone(),
				NodeTokensPlugin::default(),
				BuildTemplatesPlugin::default(),
				CodegenNativePlugin::default(),
			))
			.run_async(FsApp::default().runner())
			.await
			.anyhow()
	}
}
