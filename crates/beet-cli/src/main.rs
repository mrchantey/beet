#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_cli::prelude::*;
use clap::Parser;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[tokio::main]
async fn main() {
	if let Err(err) = Cli::parse().command.run().await {
		panic!("Error: {:?}", err);
	}
}
