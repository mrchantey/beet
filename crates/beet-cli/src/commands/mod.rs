use bevy::prelude::*;
use clap::Subcommand;
mod run_build;
pub use run_build::*;


#[derive(Clone, Subcommand)]
pub enum Commands {
	Build(RunBuild),
}
impl Plugin for Commands {
	fn build(&self, app: &mut App) {
		match self {
			Commands::Build(cmd) => app.add_plugins(cmd.clone()),
		};
	}
}
