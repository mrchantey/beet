#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use sweet::prelude::*;
use sweet_cli::prelude::*;

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
	BenchAssert(BenchAssert),
	TestServer(TestServer),
	TestWasm(TestWasm),
	Serve(Server),
	Watch(FsWatchCmd),
	Mod(AutoMod),
}

#[tokio::main]
async fn main() -> Result<()> {
	match Cli::parse().command {
		Commands::BenchAssert(cmd) => cmd.run(),
		Commands::TestServer(cmd) => cmd.run(),
		Commands::TestWasm(cmd) => cmd.run(),
		Commands::Serve(cmd) => cmd.run().await,
		Commands::Watch(cmd) => cmd.run_and_watch().await,
		Commands::Mod(cmd) => cmd.run().await,
	}
}
