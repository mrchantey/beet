use beet_core::prelude::*;
use beet_rsx::prelude::*;

use crate::actions::ChildProcess;

#[construct]
pub fn SstCommand(
	cmd: SstSubcommand,
	pkg_config: Res<PackageConfig>,
) -> Result<impl Bundle> {
	let sst_dir = WsPathBuf::default().join("infra");
	ChildProcess::new("npx")
		.current_dir(sst_dir.to_string())
		.arg("sst")
		.arg(cmd.to_cmd())
		.arg("--stage")
		.arg(pkg_config.stage())
		.xok()
	// 	// 	"🌱 Running SST command: \n   {cmd:?}\n🌱 Interrupting this step may result in dangling resources"
}

impl SstCommand {
	/// Creates an `SstCommand` bundle for the given subcommand.
	pub fn new(cmd: SstSubcommand) -> impl Bundle {
		(Name::new("SST Command"), SstCommand { cmd })
	}
}


/// Represents the available subcommands for the SST CLI.
#[allow(unused)]
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
	#[allow(unused)]
	fn to_cmd(&self) -> &str {
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
