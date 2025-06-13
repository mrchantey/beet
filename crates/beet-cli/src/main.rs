#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use anyhow::Result;
use beet::prelude::*;
use beet_cli::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;
use clap::Subcommand;

fn main() {
	if let AppExit::Error(err) = App::new()
		.add_plugins((
			LogPlugin {
				level: bevy::log::Level::DEBUG,
				..default()
			},
			Cli::parse(),
		))
		.set_runner(FsApp::default().runner())
		.run()
	{
		std::process::exit(err.get() as i32);
	}
}


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
	Test(CargoTest),
	TestServer(TestServer),
	TestWasm(TestWasm),
	Run(CargoRun),
	Serve(Server),
	Watch(FsWatchCmd),
	Mod(AutoMod),
}

#[tokio::main]
async fn main() -> Result<()> {
	match Cli::parse().command {
		Commands::BenchAssert(cmd) => cmd.run(),
		Commands::Run(cmd) => cmd.run().await,
		Commands::Test(cmd) => cmd.run().await,
		Commands::TestServer(cmd) => cmd.run(),
		Commands::TestWasm(cmd) => cmd.run(),
		Commands::Serve(cmd) => cmd.run().await,
		Commands::Watch(cmd) => cmd.run_and_watch().await,
		Commands::Mod(cmd) => cmd.run().await,
	}
}





impl Plugin for Cli {
	fn build(&self, app: &mut App) { app.add_plugins(self.commands.clone()); }
}
