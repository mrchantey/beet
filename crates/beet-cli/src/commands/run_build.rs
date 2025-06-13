use crate::prelude::*;
use beet::prelude::*;
use bevy::log::LogPlugin;
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
	pub build_args: BuildArgs,
}

impl RunBuild {
	pub fn run(self) -> anyhow::Result<()> {
		App::new()
			.add_plugins(LogPlugin {
				level: bevy::log::Level::DEBUG,
				..default()
			})
			.insert_resource(self.build_cmd.clone())
			.add_plugins(self.build_args.clone())
			.set_runner(FsApp::default().runner())
			.run()
			.anyhow()
	}
}
