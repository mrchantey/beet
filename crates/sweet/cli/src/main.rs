#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use clap::Parser;
use clap::Subcommand;
use sweet_cli::prelude::*;

/// 🤘 Sweet CLI 🤘
///
/// A sweet as test runner.
#[derive(Parser)]
#[command(version)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	TestServer(TestServer),
	TestWasm(TestWasm),
	Run(CargoRun),
	Test(CargoTest),
}

#[tokio::main]
async fn main() -> Result {
	match Cli::parse().command {
		Commands::TestServer(cmd) => cmd.run(),
		Commands::TestWasm(cmd) => cmd.run(),
		Commands::Run(cmd) => cmd.run().await,
		Commands::Test(cmd) => cmd.run().await,
	}
}
