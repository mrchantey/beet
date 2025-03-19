mod cargo_cmd;
mod build_step;
pub use cargo_cmd::*;
mod build_cmd;
pub use build_cmd::*;
pub use build_step::*;
mod build_binaries;
pub use build_binaries::*;
mod deploy;
mod watch;
pub use deploy::*;
pub use watch::*;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
	Watch(Watch),
	Deploy(Deploy),
}

impl Commands {
	pub async fn run(self) -> Result<()> {
		match self {
			Commands::Watch(cmd) => cmd.run().await,
			Commands::Deploy(cmd) => cmd.run(),
		}
	}
}
