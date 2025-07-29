use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct RunBuild {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	pub(crate) build_cmd: CargoBuildCmd,
	/// Location of the beet.toml config file
	#[arg(long)]
	pub(crate) beet_config: Option<PathBuf>,
	/// Run a simple file server in this process instead of
	/// spinning up the native binary with the --server feature
	#[arg(long = "static")]
	pub(crate) r#static: bool,
	/// Only execute the provided build steps,
	/// options are "routes", "snippets", "client-islands", "compile-server", "export-ssg", "compile-wasm", "run-server"
	#[arg(long, value_delimiter = ',', value_parser = parse_flags)]
	pub(crate) only: Vec<BuildFlag>,
}

fn parse_flags(s: &str) -> Result<BuildFlag, String> { BuildFlag::from_str(s) }

#[derive(PartialEq)]
pub enum RunMode {
	Once,
	Watch,
}



impl RunBuild {
	pub fn load_binary_name(&self) -> Result<String> {
		let manifest = CargoManifest::load()?;
		let package_name = manifest.package_name();
		Ok(self.build_cmd.binary_name(package_name))
	}

	pub fn workspace_config(&self) -> Result<WorkspaceConfig> {
		todo!("load from toml, bsn or cli args");
		// Ok(WorkspaceConfig::default())
	}

	pub async fn run(self, run_mode: RunMode) -> Result {
		let mut app = App::new();
		let config = self.workspace_config()?;

		let build_flags = if self.only.is_empty() {
			BuildFlags::All
		} else {
			BuildFlags::Only(self.only)
		};



		app.insert_resource(build_flags)
			.insert_resource(self.build_cmd)
			.insert_resource(config)
			.add_plugins(BuildPlugin::default());


		LaunchRunner {
			watch: run_mode == RunMode::Watch,
		}
		.run(app)
		.into_result()
	}
}
