use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;

// TODO probably integrate with RunBuild, and just nest
#[derive(Debug, Clone, Parser)]
pub struct LoadBeetConfig {
	/// Location of the beet.toml config file
	#[arg(long)]
	pub beet_config: Option<PathBuf>,
	/// Run a simple file server in this process instead of
	/// spinning up the native binary with the --server feature
	#[arg(long = "static")]
	pub r#static: bool,
	/// Only execute the provided build steps,
	/// options are `templates`, `routes`, `server`, `static`, `wasm`
	#[arg(long, value_delimiter = ',', value_parser = parse_build_only)]
	pub only: Vec<BuildOnly>,
}

impl Plugin for LoadBeetConfig {
	fn build(&self, app: &mut App) {
		let config =
			BeetConfig::try_load_or_default(self.beet_config.as_deref())
				.unwrap_or_exit();
		config.build(app, &self.only);
	}
}


fn parse_build_only(s: &str) -> Result<BuildOnly, String> {
	BuildOnly::from_str(s)
}
