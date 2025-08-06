use crate::utils::CargoManifest;
use beet_utils::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use heck::ToKebabCase;
use std::process::Command;

/// A very light wrapper around sst, adapted to a few beet specific conventions
/// like ensuring commands are run in the `infra` directory.
#[derive(Debug, Clone, Parser, Resource)]
pub struct SstConfig {
	/// The default stage for development.
	#[arg(long, default_value = "dev")]
	pub dev_stage: String,
	/// The default stage for production.
	#[arg(long, default_value = "prod")]
	pub prod_stage: String,
	/// Optionally specify the sst stage name used, which will otherwise be inferred
	/// from debug/release build, defaulting to `dev` or `prod`.
	#[arg(long)]
	pub stage: Option<String>,
}

impl Default for SstConfig {
	fn default() -> Self {
		SstConfig {
			dev_stage: "dev".to_string(),
			prod_stage: "prod".to_string(),
			stage: None,
		}
	}
}

pub fn deploy_sst(
	build_cmd: Res<CargoBuildCmd>,
	sst_config: Res<SstConfig>,
) -> Result {
	sst_config.run_sst(&build_cmd, "deploy")
}


impl SstConfig {
	/// Specify the stage name used. if the build command specifies release,
	/// this defaults to `prod`, otherwise `dev`.
	pub fn stage(&self, build_cmd: &CargoBuildCmd) -> &str {
		self.stage.as_ref().unwrap_or_else(|| {
			if build_cmd.release {
				&self.prod_stage
			} else {
				&self.dev_stage
			}
		})
	}

	/// binary-resource-stage convention to match
	/// sst.config.ts -> new sst.aws.Function(`..`, {name: `THIS_FIELD` }),
	pub fn lambda_func_name(
		&self,
		build_cmd: &CargoBuildCmd,
		manifest: &CargoManifest,
	) -> String {
		let binary_name = build_cmd.binary_name(manifest.package_name());
		let stage = self.stage(build_cmd);
		format! {"{}-lambda-{}",binary_name.to_kebab_case(),stage}
	}

	fn run_sst(&self, build_cmd: &CargoBuildCmd, subcommand: &str) -> Result {
		let mut args = vec!["sst", subcommand];
		let stage = self.stage(build_cmd);
		args.push("--stage");
		args.push(&stage);

		let sst_dir = std::env::current_dir()?.join("infra").canonicalize()?;
		let mut cmd = Command::new("npx");
		cmd.current_dir(sst_dir).args(args);
		// .arg("--config")
		// .arg("infra/sst.config.ts")

		println!(
			"🌱 Running SST command: \n   {cmd:?}\n🌱 Interrupting this step may result in dangling resources"
		);
		cmd.status()?.exit_ok()?.xok()
	}
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::fs::CargoBuildCmd;
	use sweet::prelude::*;

	#[test]
	fn works() {
		SstConfig::default()
			.lambda_func_name(
				&CargoBuildCmd::default(),
				&CargoManifest::load().unwrap(),
			)
			.xpect()
			.to_be("beet-lambda-dev");
	}
}
