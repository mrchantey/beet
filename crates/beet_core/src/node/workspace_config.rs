use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::path::Path;

/// Config for the scene containing all information that can be statically extracted
/// from files, including html, parsed styles etc.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct WorkspaceConfig {
	/// Filter for extracting snippets,
	/// excludes 'target' and 'node_modules' directories by default
	#[reflect(ignore)] // TODO reflect GlobFilter
	pub filter: GlobFilter,
	/// The root directory for extracting snippets
	pub root_dir: WsPathBuf,
	/// The output location for the generated template scene file
	pub snippets_dir: WsPathBuf,
	/// Location of the html directory, defaults to 'target/client'
	pub html_dir: WsPathBuf,
	/// Directory for temp static files like client islands.
	pub client_islands_path: WsPathBuf,
}
impl Default for WorkspaceConfig {
	fn default() -> Self {
		Self {
			filter: GlobFilter::default()
				.with_exclude("*/target/*")
				.with_exclude("*/codegen/*")
				.with_exclude("*/.cache/*")
				.with_exclude("*/node_modules/*"),
			root_dir: {
				#[cfg(test)]
				{
					WsPathBuf::new("crates/beet_router/src/test_site")
				}
				#[cfg(not(test))]
				{
					WsPathBuf::default()
				}
			},
			snippets_dir: WsPathBuf::new("target/snippets"),
			html_dir: WsPathBuf::new("target/client"),
			client_islands_path: WsPathBuf::new("target/client_islands.ron"),
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
	pub fn rsx_snippet_path(&self, idx: &SnippetRoot) -> WsPathBuf {
		let mut path = idx.file.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name =
			format!("{}:{}.rsx.ron", file_stem, idx.start.to_string());
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	/// Create a file path in the format of `path/to/file.ron`,
	/// we need the index because some files may have multiple LangSnippets
	/// and we dont always have the span.
	/// using [`Self::snippets_dir`] as the base.
	pub fn lang_snippet_path(&self, path: &WsPathBuf, index: u64) -> WsPathBuf {
		let mut path = path.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name = format!("{}-{}.lang.ron", file_stem, index);
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
