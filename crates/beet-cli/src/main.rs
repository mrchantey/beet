#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_cli::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use clap::Subcommand;

/// 🌱 Beet CLI 🌱
///
/// Various commands for testing and serving files.
#[derive(Parser)]
#[command(version)]
struct Cli {
	#[command(subcommand)]
	command: SubCommands,
}

#[derive(Subcommand)]
enum SubCommands {
	Build(RunBuild),
	New(RunNew),
	Agent(AgentCmd),
	ExportPdf(ExportPdf),
}

#[tokio::main]
async fn main() -> Result {
	match Cli::parse().command {
		SubCommands::Build(cmd) => cmd.run().await,
		SubCommands::New(cmd) => cmd.run().await,
		SubCommands::Agent(cmd) => cmd.run().await,
		SubCommands::ExportPdf(cmd) => cmd.run().await,
	}
}
