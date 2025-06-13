#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use anyhow::Result;
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
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Build(RunBuild),
}

#[tokio::main]
async fn main() -> Result<()> {
	match Cli::parse().command {
		Commands::Build(cmd) => cmd.run(),
	}
}
