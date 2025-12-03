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
	// Run(RunCmd),
	Build(RunBuild),
	New(RunNew),
	Agent(AgentCmd),
	ExportPdf(ExportPdf),
	#[cfg(feature = "qrcode")]
	Qrcode(QrCodeCmd),
}

#[tokio::main]
async fn main() -> Result {
	match Cli::parse().command {
		// SubCommands::Run(cmd) => cmd.run().await,
		SubCommands::Build(cmd) => cmd.run().await,
		SubCommands::New(cmd) => cmd.run().await,
		SubCommands::Agent(cmd) => cmd.run().await,
		SubCommands::ExportPdf(cmd) => cmd.run().await,
		#[cfg(feature = "qrcode")]
		SubCommands::Qrcode(cmd) => cmd.run().await,
	}
}
