use crate::prelude::*;
use anyhow::Result;
use beet_common::node::HtmlConstants;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;
use sweet::prelude::*;



/// Config file usually located at `beet.toml`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeetConfig {
	/// Output location for generated static files, ie html, css, wasm
	templates_config: BuildFileTemplates,
	html_constants: HtmlConstants,
}

impl BeetConfig {
	/// 1. Attempt to load the config from the specified path
	/// 2. Attempt to load from the default location `beet.toml`
	/// 3. Fall back to the default config if not found
	/// ## Errors
	/// If a path is specified and the file is not found
	pub fn load(path: Option<&Path>) -> Result<Self> {
		path
			// if a config is specified and not found, exit
			.map(|path| BeetConfig::from_file(&path))
			// if no config is specified, use the default
			.unwrap_or_else(|| {
				BeetConfig::from_file("beet.toml").unwrap_or_default().xok()
			})
	}

	fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		Ok(toml::de::from_str(&ReadFile::to_string(path)?)?)
	}
}


impl Plugin for BeetConfig {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.html_constants.clone());
		app.world_mut().spawn(self.templates_config.clone());
	}
}
