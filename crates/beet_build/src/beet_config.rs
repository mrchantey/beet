use crate::prelude::*;
use anyhow::Result;
use beet_common::node::HtmlConstants;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;



/// Config file usually located at `beet.toml`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeetConfig {
	/// Output location for generated static files, ie html, css, wasm
	pub templates_config: BuildFileTemplates,
	pub html_constants: HtmlConstants,
	#[serde(rename = "file_group")]
	pub file_groups: Vec<FileGroupConfig>,
	// /// Configuration for a default site configuration.
	#[serde(flatten)]
	pub default_site_config: CodegenConfig,
}

impl BeetConfig {
	/// 1. Attempt to load the config from the specified path
	/// 2. Attempt to load from the default location `beet.toml`
	/// 3. Fall back to the default config if not found
	/// ## Errors
	/// If a path is specified and the file is not found
	pub fn try_load_or_default(path: Option<&Path>) -> Result<Self> {
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


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileGroupConfig {
	name: String,
	// #[serde(flatten)]
	// pub file_group: FileGroup,
	// #[serde(flatten)]
	// pub codegen: CodegenFile,
	// #[serde(flatten)]
	// pub map_tokens: MapFuncTokens,
}
