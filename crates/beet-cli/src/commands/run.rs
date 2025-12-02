use beet::prelude::*;
use clap::Parser;

/// Run the beet project
#[derive(Debug, Clone, Parser)]
pub struct RunCmd {
	/// The package to run
	#[arg(short, long)]
	pub package: Option<String>,
	/// Any additional cargo args to run
	#[arg(long)]
	pub config_cargo_args: Option<String>,
	/// Exclude the 'config' feature, package name etc when
	/// building the config binary
	#[arg(long)]
	pub config_no_default_args: bool,
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
