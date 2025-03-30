mod cargo_cmd;
mod run_build;
pub use cargo_cmd::*;
mod cargo_build_cmd;
pub use cargo_build_cmd::*;
mod build_steps;
pub use build_steps::*;
mod run_deploy;
mod run_watch;
pub use run_build::*;
pub use run_deploy::*;
pub use run_watch::*;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
	Watch(RunWatch),
	Deploy(RunDeploy),
	Build(RunBuild),
}

impl Commands {
	pub async fn run(self) -> Result<()> {
		match self {
			Commands::Watch(cmd) => cmd.run().await,
			Commands::Deploy(cmd) => cmd.run(),
			Commands::Build(cmd) => cmd.run(),
		}
	}
}
