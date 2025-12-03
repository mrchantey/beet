use beet::prelude::*;
use clap::Parser;

/// Run the beet project
#[derive(Debug, Clone, Parser)]
pub struct RunCmd {
}


impl RunCmd {
	pub async fn run(self) -> Result {
		App::new()
			.add_plugins(BuildPlugin::default())
			.set_runner(LaunchRunner::runner)
			.run()
			.into_result()
	}
}


fn default_runner() -> impl Bundle { (config_step(),) }

#[template]
fn ConfigStep() -> impl Bundle {
	
	
	rsx!{
		
	}	
}
