#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_cli::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use clap::Subcommand;

/// Sweet CLI
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
	Run(RunBuild),
	Build(RunBuild),
}

#[tokio::main]
async fn main() -> Result {
	init_tracing(bevy::log::Level::DEBUG);
	match Cli::parse().command {
		SubCommands::Build(cmd) => cmd.build().await,
		SubCommands::Run(cmd) => cmd.run().await,
	}
}
