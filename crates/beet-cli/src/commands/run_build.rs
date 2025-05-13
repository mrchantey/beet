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
	#[command(flatten)]
	pub build_args: BuildArgs,
	#[command(flatten)]
	pub build_template_maps: BuildTemplateMaps,
	/// used by watch command only, inserts server step after native build
	#[arg(long, default_value_t = false)]
	pub server: bool,
}

impl Plugin for RunBuild {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.build_args.clone())
			.insert_resource(self.build_cmd.clone());
	}
}
