use crate::prelude::*;
use anyhow::Result;
use beet_bevy::prelude::AppExt;
use beet_common::node::HtmlConstants;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use std::str::FromStr;



/// Config file usually located at `beet.toml`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeetConfig {
	#[serde(default)]
	pub html_constants: HtmlConstants,
	#[serde(default)]
	pub template_scene: BuildFileTemplates,
	#[serde(default)]
	pub codegen_native: CodegenNativeConfig,
	#[serde(default)]
	pub codegen_wasm: CodegenWasmConfig,
}

impl BeetConfig {
	/// 1. Attempt to load the config from the specified path
	/// 2. Attempt to load from the default location `beet.toml`
	/// 3. Fall back to the default config if not found
	/// ## Errors
	/// If a path is specified and the file is not found
	pub fn try_load_or_default(path: Option<&Path>) -> Result<Self> {
		match path {
			Some(path) => BeetConfig::from_file(path),
			None => {
				let default_path = Path::new("beet.toml");
				if default_path.exists() {
					BeetConfig::from_file(default_path)
				} else {
					Ok(BeetConfig::default())
				}
			}
		}
	}

	fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		Ok(toml::de::from_str(&ReadFile::to_string(path)?)?)
	}
	/// Insert resources and plugins to reflect the [`only`] options,
	/// inserting all if [`only`] is empty.
	pub fn build(self, app: &mut App, only: &Vec<BuildOnly>) {
		let all = only.is_empty();
		app.insert_resource(self.html_constants);
		app.init_resource::<HtmlConstants>();
		if all || only.contains(&BuildOnly::Templates) {
			app.world_mut().spawn(self.template_scene);
		}
		if all || only.contains(&BuildOnly::Native) {
			app.add_non_send_plugin(self.codegen_native);
		}
		if all || only.contains(&BuildOnly::Wasm) {
			app.add_non_send_plugin(self.codegen_wasm);
		}
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

impl FromStr for BuildOnly {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"templates" => Ok(BuildOnly::Templates),
			"native" => Ok(BuildOnly::Native),
			"server" => Ok(BuildOnly::Server),
			"static" => Ok(BuildOnly::Static),
			"wasm" => Ok(BuildOnly::Wasm),
			_ => Err(format!("Unknown only field: {}", s)),
		}
	}
}
