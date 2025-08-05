use beet::prelude::*;
use clap::Parser;

/// Build the project
#[derive(Debug, Clone, Parser)]
pub struct RunBuild {
	/// 🦀 the commands that will be used to build the binary 🦀
	#[command(flatten)]
	pub(crate) build_cmd: CargoBuildCmd,
}

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

	#[allow(unused)]
	pub async fn run(self, _run_mode: RunMode) -> Result {
		todo!("pass run mode");
		let mut app = App::new();

		app.add_plugins(BuildPlugin::default())
			.set_runner(LaunchRunner::runner)
			.run()
			.into_result()
	}
}
