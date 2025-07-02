use beet_common::node::HtmlConstants;
use beet_common::node::MacroIdx;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::path::Path;

/// Collection of resources to be inserted into the app.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateConfig {
	#[cfg_attr(feature = "serde", serde(default))]
	pub html_constants: HtmlConstants,
	#[cfg_attr(feature = "serde", serde(default))]
	pub workspace: WorkspaceConfig,
}

impl Plugin for TemplateConfig {
	#[rustfmt::skip]
	fn build(&self, app: &mut App) {
		app
			.insert_resource(self.html_constants.clone())
			.insert_resource(self.workspace.clone())
			;
	}
}

impl TemplateConfig {
	pub fn default_config_path(&self) -> AbsPathBuf {
		WsPathBuf::new("beet.toml").into_abs()
	}
}

/// Config for the scene containing all information that can be statically extracted
/// from files, including html, parsed styles etc.
#[derive(Debug, Clone, PartialEq, Resource)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorkspaceConfig {
	/// Filter for files that should be parsed,
	/// excludes 'target' and 'node_modules' directories by default
	#[cfg_attr(feature = "serde", serde(default = "default_filter"))]
	pub filter: GlobFilter,
	/// The root directory for files including templates
	#[cfg_attr(feature = "serde", serde(default = "default_root_dir"))]
	pub root_dir: WsPathBuf,
	/// The location for the generated template scene file
	#[cfg_attr(feature = "serde", serde(default = "default_snippets_dir"))]
	pub snippets_dir: WsPathBuf,
	/// Location of the html directory, defaults to 'target/client'
	#[cfg_attr(feature = "serde", serde(default = "default_html_dir"))]
	pub html_dir: WsPathBuf,
	/// Directory for temp static files like client islands.
	#[cfg_attr(
		feature = "serde",
		serde(default = "default_client_islands_path")
	)]
	pub client_islands_path: WsPathBuf,
}
#[allow(unused)]
fn default_filter() -> GlobFilter {
	GlobFilter::default()
		.with_include("*/crates/beet_design/src/**/*")
		.with_include("*/crates/beet_site/src/**/*")
		.with_include("*/crates/beet_router/src/test_site/**/*")
		.with_exclude("*/target/*")
		.with_exclude("*/.cache/*")
		.with_exclude("*/node_modules/*")
}
#[allow(unused)]
fn default_root_dir() -> WsPathBuf {
	#[cfg(test)]
	{
		WsPathBuf::new("crates/beet_router/src/test_site")
	}
	#[cfg(not(test))]
	{
		WsPathBuf::default()
	}
}
#[allow(unused)]
fn default_snippets_dir() -> WsPathBuf { WsPathBuf::new("target/snippets") }
#[allow(unused)]
fn default_html_dir() -> WsPathBuf { WsPathBuf::new("target/client") }
#[allow(unused)]
fn default_client_islands_path() -> WsPathBuf {
	WsPathBuf::new("target/client_islands.ron")
}

impl Default for WorkspaceConfig {
	fn default() -> Self {
		Self {
			filter: default_filter(),
			root_dir: default_root_dir(),
			snippets_dir: default_snippets_dir(),
			html_dir: default_html_dir(),
			client_islands_path: default_client_islands_path(),
		}
	}
}

impl WorkspaceConfig {
	pub fn test_site() -> Self {
		let mut this = Self::default();
		this.root_dir = WsPathBuf::new("crates/beet_router/src/test_site");
		this
	}

	pub fn snippets_dir(&self) -> &WsPathBuf { &self.snippets_dir }

	/// Create a file path in the format of `path/to/file:line:col.rs`,
	/// using [`Self::snippets_dir`] as the base.
	pub fn rsx_snippet_path(&self, idx: &MacroIdx) -> WsPathBuf {
		let mut path = idx.file.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name =
			format!("rsx-{}:{}.ron", file_stem, idx.start.to_string());
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	/// Create a file path in the format of `path/to/file.ron`,
	/// we need the index because some files may have multiple LangSnippets
	/// and we dont always have the span.
	/// using [`Self::snippets_dir`] as the base.
	pub fn lang_snippet_path(
		&self,
		path: &WsPathBuf,
		index: u64,
	) -> WsPathBuf {
		let mut path = path.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let extension = "ron";
		let snippet_file_name =
			format!("lang-{}-{}.{}", file_stem, index, extension);
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		self.filter.passes(path)
	}
	pub fn get_files(&self) -> Result<Vec<AbsPathBuf>, FsError> {
		ReadDir::files_recursive(&self.root_dir.into_abs())?
			.into_iter()
			.filter(|path| self.filter.passes(path))
			.map(|path| AbsPathBuf::new(path))
			.collect()
	}
}



/// The beet.toml file can be loaded partially, for example
/// [`TemplateConfig`] is only a subset of the settings. This  
/// type provides convenience methods to load any part of the config
/// file.
pub struct BeetConfigFile;


#[cfg(feature = "serde")]
impl BeetConfigFile {
	/// 1. Attempt to load the config from the specified path
	/// 2. Attempt to load from the default location `beet.toml`
	/// 3. Fall back to the default config if not found
	/// ## Errors
	/// If a path is specified and the file is not found
	pub fn try_load_or_default<T: Default + serde::de::DeserializeOwned>(
		path: Option<&Path>,
	) -> Result<T> {
		match path {
			Some(path) => Self::from_file(path),
			None => {
				let default_path = Path::new("beet.toml");
				if default_path.exists() {
					Self::from_file(default_path)
				} else {
					Ok(default())
				}
			}
		}
	}

	fn from_file<T: serde::de::DeserializeOwned>(
		path: impl AsRef<Path>,
	) -> Result<T> {
		Ok(beet_common::exports::toml::de::from_str(
			&ReadFile::to_string(path)?,
		)?)
	}
}
