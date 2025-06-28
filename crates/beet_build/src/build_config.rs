use crate::prelude::*;
use anyhow::Result;
use beet_bevy::prelude::AppExt;
use beet_bevy::prelude::NonSendPlugin;
use beet_template::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;




/// Config file usually located at `beet.toml`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildConfig {
	#[serde(flatten)]
	pub template_config: TemplateConfig,
	pub route_codegen: RouteCodegenConfig,
	pub client_island_codegen: ClientIslandCodegenConfig,
}

impl BuildConfig {
	/// 1. Attempt to load the config from the specified path
	/// 2. Attempt to load from the default location `beet.toml`
	/// 3. Fall back to the default config if not found
	/// ## Errors
	/// If a path is specified and the file is not found
	pub fn try_load_or_default(path: Option<&Path>) -> Result<Self> {
		match path {
			Some(path) => BuildConfig::from_file(path),
			None => {
				let default_path = Path::new("beet.toml");
				if default_path.exists() {
					BuildConfig::from_file(default_path)
				} else {
					Ok(BuildConfig::default())
				}
			}
		}
	}

	fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		Ok(toml::de::from_str(&ReadFile::to_string(path)?)?)
	}
}

impl NonSendPlugin for BuildConfig {
	fn build(self, app: &mut App) {
		app.add_plugins(self.template_config)
			.add_non_send_plugin(self.route_codegen)
			.add_non_send_plugin(self.client_island_codegen);
	}
}
