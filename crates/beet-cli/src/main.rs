#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use anyhow::Result;
use beet::prelude::*;
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
	Serve(Server),
	Watch(FsWatchCmd),
	Mod(AutoMod),
	Build(RunBuild),
}

#[tokio::main]
async fn main() -> Result<()> {
	match Cli::parse().command {
		Commands::Serve(cmd) => cmd.run().await,
		Commands::Watch(cmd) => cmd.run_and_watch().await,
		Commands::Mod(cmd) => cmd.run().await,
		Commands::Build(cmd) => cmd.run(),
	}
}
