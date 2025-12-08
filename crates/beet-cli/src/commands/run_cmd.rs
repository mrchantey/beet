use beet::prelude::*;
use clap::Parser;

/// Run the beet project
#[derive(Debug, Clone, Parser)]
pub struct RunCmd {}


impl RunCmd {
	pub async fn run(self) -> Result {
		App::new()
			// TODO new build plugin
			// .add_plugins(BuildPlugin::default())
			// .set_runner(LaunchRunner::runner)
			.run()
			.into_result()
	}
}
