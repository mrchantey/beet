#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use anyhow::Result;
use beet_cli::prelude::*;
use clap::Parser;
use clap::Subcommand;


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}
impl Cli {
	pub async fn run(self) -> Result<()> {
		match self.command {
			Commands::Watch(cmd) => cmd.run().await,
			Commands::Deploy(cmd) => cmd.run(),
		}
	}
}

#[derive(Subcommand)]
enum Commands {
	Watch(Watch),
	Deploy(Deploy),
}



#[tokio::main]
async fn main() {
	if let Err(err) = Cli::parse().run().await {
		panic!("Error: {:?}", err);
	}
}
