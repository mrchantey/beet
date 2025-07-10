use beet::prelude::*;
use clap::Parser;
use heck::ToKebabCase;
use tokio::process::Command;



/// A very light wrapper around sst, adapted to a few beet specific conventions
/// like ensuring commands are run in the `infra` directory.
#[derive(Parser)]
pub struct RunInfra {
	/// The subcommand to run (deploy or remove)
	#[arg(value_enum, default_value = "deploy")]
	subcommand: SstSubcommand,
	/// The stage to use, defaults to `dev`
	#[arg(long, default_value = "dev")]
	stage: String,
}

/// Represents the available subcommands for the SST CLI.
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum SstSubcommand {
	/// Initialize a new project
	Init,
	/// Run in development mode
	Dev,
	/// Deploy your application
	Deploy,
	/// See what changes will be made
	Diff,
	/// Add a new provider
	Add,
	/// Install all the providers
	Install,
	/// Manage secrets
	Secret,
	/// Run a command with linked resources
	Shell,
	/// Remove your application
	Remove,
	/// Clear any locks on the app state
	Unlock,
	/// Print the version of the CLI
	Version,
	/// Upgrade the CLI
	Upgrade,
	/// Manage telemetry settings
	Telemetry,
	/// Refresh the local app state
	Refresh,
	/// Manage state of your app
	State,
	/// Generate certificate for the Console
	Cert,
	/// Start a tunnel
	Tunnel,
	/// Generates a diagnostic report
	Diagnostic,
}
impl SstSubcommand {
	/// Returns the name of the subcommand as a string.
	pub fn as_str(&self) -> &str {
		match self {
			SstSubcommand::Init => "init",
			SstSubcommand::Dev => "dev",
			SstSubcommand::Deploy => "deploy",
			SstSubcommand::Diff => "diff",
			SstSubcommand::Add => "add",
			SstSubcommand::Install => "install",
			SstSubcommand::Secret => "secret",
			SstSubcommand::Shell => "shell",
			SstSubcommand::Remove => "remove",
			SstSubcommand::Unlock => "unlock",
			SstSubcommand::Version => "version",
			SstSubcommand::Upgrade => "upgrade",
			SstSubcommand::Telemetry => "telemetry",
			SstSubcommand::Refresh => "refresh",
			SstSubcommand::State => "state",
			SstSubcommand::Cert => "cert",
			SstSubcommand::Tunnel => "tunnel",
			SstSubcommand::Diagnostic => "diagnostic",
		}
	}
}

impl RunInfra {
	/// The name of the lambda function matching the one
	/// in sst.config.ts -> new sst.aws.Function(`..`, {name: `THIS_FIELD` }),
	pub fn lambda_func_name(binary_name: &str, stage: &str) -> String {
		format! {"{}-{}-lambda",binary_name.to_kebab_case(),stage}
	}

	pub async fn run(&self) -> Result {
		let mut args = vec!["sst", self.subcommand.as_str()];
		args.push("--stage");
		args.push(&self.stage);

		let sst_dir = std::env::current_dir()?.join("infra").canonicalize()?;
		let mut cmd = Command::new("npx");
		cmd.current_dir(sst_dir).args(args);
		// .arg("--config")
		// .arg("infra/sst.config.ts")

		println!(
			"🌱 Running SST command: \n   {cmd:?}\n🌱 Interrupting this step may result in dangling AWS Resources"
		);
		cmd.status().await?.exit_ok()?.xok()
	}
}
