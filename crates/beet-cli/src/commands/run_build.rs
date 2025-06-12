use crate::prelude::*;
use bevy::prelude::*;
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
	pub build_args: BuildArgs,
}

impl Plugin for RunBuild {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.build_cmd.clone())
			.add_plugins(self.build_args.clone());
	}
}
