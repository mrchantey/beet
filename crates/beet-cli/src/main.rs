#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::init_pretty_tracing;
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
	Run(RunBuild),
	Build(RunBuild),
	Deploy(RunDeploy),
	Infra(RunInfra),
}

#[tokio::main]
async fn main() -> Result {
	init_pretty_tracing(bevy::log::Level::DEBUG);
	match Cli::parse().command {
		SubCommands::Build(cmd) => cmd.run(RunMode::Once).await,
		SubCommands::Run(cmd) => cmd.run(RunMode::Watch).await,
		SubCommands::Deploy(cmd) => cmd.run().await,
		SubCommands::Infra(cmd) => cmd.run().await,
	}
}
