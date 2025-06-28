use beet_common::node::HtmlConstants;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::path::Path;

/// Collection of resources to be inserted into the app.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateConfig {
	#[cfg_attr(feature = "serde", serde(default))]
	pub html_constants: HtmlConstants,
	#[cfg_attr(feature = "serde", serde(flatten))]
	pub server_output_config: ServerOutputConfig,
	#[cfg_attr(feature = "serde", serde(default))]
	pub static_scene_config: StaticSceneConfig,
}

impl Plugin for TemplateConfig {
	#[rustfmt::skip]
	fn build(&self, app: &mut App) {
		app
			.insert_resource(self.html_constants.clone())
			.insert_resource(self.server_output_config.clone())
			.insert_resource(self.static_scene_config.clone())
			;
	}
}


#[derive(Debug, Default, Clone, PartialEq, Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ServerOutputConfig {
	/// Location of the html directory, defaults to 'target/client'
	#[cfg_attr(feature = "serde", serde(default = "default_html_dir"))]
	pub html_dir: WsPathBuf,
}
#[allow(unused)]
fn default_html_dir() -> WsPathBuf { WsPathBuf::new("target/client") }


/// Config for the scene containing all information that can be statically extracted
/// from files, including html, parsed styles etc.
#[derive(Debug, Clone, PartialEq, Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StaticSceneConfig {
	/// Filter for files that should be parsed,
	/// excludes 'target' and 'node_modules' directories by default
	filter: GlobFilter,
	/// The root directory for files including templates
	root_dir: WsPathBuf,
	/// The location for the generated template scene file
	scene_file: WsPathBuf,
}

impl Default for StaticSceneConfig {
	fn default() -> Self {
		Self {
			filter: GlobFilter::default()
				// TODO move to beet.toml
				.with_include("*/crates/beet_design/src/**/*")
				.with_include("*/crates/beet_site/src/**/*")
				.with_include("*/crates/beet_router/src/test_site/**/*")
				.with_exclude("*/target/*")
				.with_exclude("*/.cache/*")
				.with_exclude("*/node_modules/*"),
			scene_file: WsPathBuf::new("target/template_scene.ron"),
			#[cfg(test)]
			root_dir: WsPathBuf::new("crates/beet_router/src/test_site"),
			#[cfg(not(test))]
			root_dir: WsPathBuf::default(),
		}
	}
}

impl StaticSceneConfig {
	pub fn test_site() -> Self {
		let mut this = Self::default();
		this.root_dir = WsPathBuf::new("crates/beet_router/src/test_site");
		this
	}

	pub fn scene_file(&self) -> &WsPathBuf { &self.scene_file }

	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		self.filter.passes(path)
	}
	pub fn get_files(&self) -> Result<Vec<WsPathBuf>, FsError> {
		ReadDir::files_recursive(&self.root_dir.into_abs())?
			.into_iter()
			.filter(|path| self.filter.passes(path))
			.map(|path| WsPathBuf::new_cwd_rel(path))
			.collect()
	}
}
