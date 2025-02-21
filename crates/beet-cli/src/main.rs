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

#[derive(Subcommand)]
enum Commands {
	ServeHtml(ServeHtml),
	WatchTemplates(WatchTemplates),
}
#[tokio::main]
async fn main() -> Result<()> {
	match Cli::parse().command {
		Commands::ServeHtml(cmd) => cmd.run().await,
		Commands::WatchTemplates(cmd) => cmd.run().await,
	}
}
