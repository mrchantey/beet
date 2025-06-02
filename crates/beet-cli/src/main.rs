#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_cli::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;

fn main() {
	if let AppExit::Error(err) = App::new()
		.add_plugins((
			LogPlugin {
				level: bevy::log::Level::DEBUG,
				..default()
			},
			Cli::parse(),
		))
		.set_runner(FsAppRunner::default().into_app_runner())
		.run()
	{
		std::process::exit(err.get() as i32);
	}
}


#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	commands: beet_cli::prelude::Commands,
}




impl Plugin for Cli {
	fn build(&self, app: &mut App) { app.add_plugins(self.commands.clone()); }
}
