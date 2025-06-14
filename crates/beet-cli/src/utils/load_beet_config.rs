use beet::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use std::path::PathBuf;

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
	/// options are `templates`, `native`, `server`, `static`, `wasm`
	#[arg(long, value_delimiter = ',', value_parser = parse_build_only)]
	pub only: Vec<BuildOnly>,
}

impl Plugin for LoadBeetConfig {
	fn build(&self, app: &mut App) {
		let config =
			BeetConfig::try_load_or_default(self.beet_config.as_deref())
				.unwrap_or_exit();
		app.add_non_send_plugin(config);
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildOnly {
	Templates,
	Native,
	Server,
	Static,
	Wasm,
}


impl std::fmt::Display for BuildOnly {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BuildOnly::Templates => write!(f, "templates"),
			BuildOnly::Native => write!(f, "native"),
			BuildOnly::Server => write!(f, "server"),
			BuildOnly::Static => write!(f, "static"),
			BuildOnly::Wasm => write!(f, "wasm"),
		}
	}
}

fn parse_build_only(s: &str) -> Result<BuildOnly, String> {
	match s.to_lowercase().as_str() {
		"templates" => Ok(BuildOnly::Templates),
		"native" => Ok(BuildOnly::Native),
		"server" => Ok(BuildOnly::Server),
		"static" => Ok(BuildOnly::Static),
		"wasm" => Ok(BuildOnly::Wasm),
		_ => Err(format!(
			"Unknown build step: {}. Valid options are: templates, native, server, static, wasm",
			s
		)),
	}
}
