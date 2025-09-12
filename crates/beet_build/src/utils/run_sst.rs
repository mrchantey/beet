use beet_dom::prelude::*;
use bevy::prelude::*;
use std::process::Command;


pub fn deploy_sst(pkg_config: Res<PackageConfig>) -> Result {
	run_sst(&pkg_config, "deploy")
}



fn run_sst(pkg_config: &PackageConfig, subcommand: &str) -> Result {
	let sst_dir = std::env::current_dir()?.join("infra").canonicalize()?;
	let mut cmd = Command::new("npx");

	cmd.current_dir(sst_dir).args(vec![
		"sst",
		subcommand,
		"--stage",
		&pkg_config.stage(),
	]);
	// .arg("--config")
	// .arg("infra/sst.config.ts")

	println!(
		"🌱 Running SST command: \n   {cmd:?}\n🌱 Interrupting this step may result in dangling resources"
	);
	cmd.status()?.exit_ok()?.xok()
}

/// Represents the available subcommands for the SST CLI.
#[allow(unused)]
#[derive(clap::ValueEnum, Clone, Debug)]
enum SstSubcommand {
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
	#[allow(unused)]
	fn as_str(&self) -> &str {
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
