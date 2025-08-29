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
	#[allow(unused)]
	pub async fn run(self) -> Result {
		App::new()
			.add_plugins(BuildPlugin::default())
			.set_runner(LaunchRunner::runner)
			.run()
			.into_result()
	}
}
