#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use anyhow::Result;
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
	BenchAssert(BenchAssert),
	TestServer(TestServer),
	TestWasm(TestWasm),
	CargoRun(CargoRun),
	CargoTest(CargoTest),
}

#[tokio::main]
async fn main() -> Result<()> {
	match Cli::parse().command {
		Commands::BenchAssert(cmd) => cmd.run(),
		Commands::TestServer(cmd) => cmd.run(),
		Commands::TestWasm(cmd) => cmd.run(),
		Commands::CargoRun(cmd) => cmd.run().await,
		Commands::CargoTest(cmd) => cmd.run().await,
	}
}
