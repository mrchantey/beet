use crate::utils::CargoManifest;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use heck::ToKebabCase;
use std::process::Command;

/// Convenience methods for information that spans multiple resources
#[derive(SystemParam)]
pub struct InfraParams<'w> {
	build_cmd: Res<'w, CargoBuildCmd>,
	manifest: Res<'w, CargoManifest>,
	infra_config: Res<'w, InfraConfig>,
}

impl InfraParams<'_> {
	pub fn binary_name(&self) -> String {
		self.build_cmd.binary_name(self.manifest.package_name())
	}

	pub fn lambda_func_name(&self) -> String {
		lambda_func_name(&self.infra_config, &self.build_cmd, &self.manifest)
	}
	pub fn bucket_name(&self) -> String {
		bucket_name(&self.infra_config, &self.build_cmd, &self.manifest)
	}

	/// resouce type may be "lambda", "bucket", etc.
	pub fn resource_name(&self, resource_type: &str) -> String {
		build_resource_name(
			&self.infra_config,
			&self.build_cmd,
			&self.manifest,
			resource_type,
		)
	}
}

/// Specify the stage name used. if the build command specifies release,
/// this defaults to `prod`, otherwise `dev`.
fn stage<'a>(infra_config: &'a InfraConfig, build_cmd: &CargoBuildCmd) -> &'a str {
	infra_config.stage.as_ref().unwrap_or_else(|| {
		if build_cmd.release {
			&infra_config.prod_stage
		} else {
			&infra_config.dev_stage
		}
	})
}

fn lambda_func_name(
	infra_config: &InfraConfig,
	build_cmd: &CargoBuildCmd,
	manifest: &CargoManifest,
) -> String {
	build_resource_name(infra_config, build_cmd, manifest, "lambda")
}

fn bucket_name(
	infra_config: &InfraConfig,
	build_cmd: &CargoBuildCmd,
	manifest: &CargoManifest,
) -> String {
	build_resource_name(infra_config, build_cmd, manifest, "bucket")
}

/// binary-resource-stage convention to match
/// sst.config.ts -> new sst.aws.Function(`..`, {name: `THIS_FIELD` }),
fn build_resource_name(
	infra_config: &InfraConfig,
	build_cmd: &CargoBuildCmd,
	manifest: &CargoManifest,
	resource_name: &str,
) -> String {
	let binary_name = build_cmd
		.binary_name(manifest.package_name())
		.to_kebab_case();
	let stage = stage(infra_config, build_cmd);
	format! {"{binary_name}-{resource_name}-{stage}"}
}

pub fn deploy_sst(
	build_cmd: Res<CargoBuildCmd>,
	infra_config: Res<InfraConfig>,
) -> Result {
	run_sst(&infra_config, &build_cmd, "deploy")
}



fn run_sst(
	infra_config: &InfraConfig,
	build_cmd: &CargoBuildCmd,
	subcommand: &str,
) -> Result {
	let mut args = vec!["sst", subcommand];
	let stage = stage(infra_config, build_cmd);
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
	use super::*;
	use crate::prelude::*;
	use beet_utils::fs::CargoBuildCmd;
	use sweet::prelude::*;

	#[test]
	fn works() {
		lambda_func_name(
			&InfraConfig::default(),
			&CargoBuildCmd::default(),
			&CargoManifest::load().unwrap(),
		)
		.xpect()
		.to_be("beet-lambda-dev");
	}
}
