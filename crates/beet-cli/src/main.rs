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
		.add_plugins((MinimalPlugins, CliPlugin))
		.add_systems(Startup, cli_routes)
		.run();
}


fn cli_routes(mut commands: Commands) {
	commands.spawn((
		CliRouter,
		// this sequence type will ensure all endpoints are checked
		// even if the previous one did not match
		InfallibleSequence,
		children![
			EndpointBuilder::get().with_handler(|| Response::ok_body(
				"hello world",
				"text/plain"
			)),
			EndpointBuilder::get().with_path("foo").with_handler(|| {
				Response::ok_body(
					"<div>hello foo</div>",
					// this inserts the `content-type: text/html`  header
					"text/html",
				)
			},),
		],
	));
}
