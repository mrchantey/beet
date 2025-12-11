#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use beet::prelude::*;
use beet_cli::prelude::*;
use clap::Parser;
use clap::Subcommand;

/// 🌱 Beet CLI 🌱
///
/// Welcome to the beet cli!
#[derive(Parser)]
#[command(version)]
struct Cli {
	#[command(subcommand)]
	command: SubCommands,
}

#[derive(Subcommand)]
enum SubCommands {
	Run(RunCmd),
	Build(BuildCmd),
	New(NewCmd),
	Agent(AgentCmd),
	ExportPdf(ExportPdf),
	#[cfg(feature = "qrcode")]
	Qrcode(QrCodeCmd),
}

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, CliPlugin, LogPlugin::default()))
		.try_set_error_handler(bevy::ecs::error::panic)
		.add_systems(Startup, cli_routes)
		.run();
}


fn cli_routes(mut commands: Commands) { commands.spawn(default_cli_router()); }
