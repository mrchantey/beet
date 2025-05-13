#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet_cli::prelude::Commands;
use bevy::prelude::*;
use clap::Parser;

fn main() {
	match App::new().add_plugins(Cli::parse()).run() {
		AppExit::Success => {}
		AppExit::Error(err) => {
			std::process::exit(err.get() as i32);
		}
	}
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

impl Plugin for Cli {
	fn build(&self, app: &mut App) { app.add_plugins(self.command.clone()); }
}
