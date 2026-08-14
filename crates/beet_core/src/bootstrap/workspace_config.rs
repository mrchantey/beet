//! The statically extractable file layout of a beet workspace.

use crate::prelude::*;
use std::path::Path;

/// Config for the scene containing all information that can be statically extracted
/// from files, including html, parsed styles etc.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct WorkspaceConfig {
	/// Filter for extracting snippets.
	///
	/// Excludes 'target' and 'node_modules' directories by default.
	pub snippet_filter: GlobFilter,
	/// Files to watch, triggering a compile-and-run of the binary with
	/// the `launch` feature on change.
	pub launch_filter: GlobFilter,
	/// Location of the `launch.ron` file to save the launch scene to.
	///
	/// See `LaunchConfig::launch_file` to direct the cli to this location.
	pub launch_file: WsPathBuf,
	/// The root directory for matching [`Self::snippet_filter`] and [`Self::launch_filter`].
	pub root_dir: WsPathBuf,
	/// The output location for the generated template scene file.
	pub snippets_dir: WsPathBuf,
	/// Location of the html directory, defaults to 'target/client'.
	pub html_dir: WsPathBuf,
	/// Location of the analytics test directory, defaults to 'target/analytics'.
	pub analytics_dir: WsPathBuf,
	/// Directory for temp static files like client islands.
	pub client_islands_path: WsPathBuf,
}
impl Default for WorkspaceConfig {
	fn default() -> Self {
		Self {
			snippet_filter: GlobFilter::default()
				.with_exclude("*/target/*")
				.with_exclude("*/codegen/*")
				.with_exclude("*/.cache/*")
				.with_exclude("*/node_modules/*"),
			launch_filter: GlobFilter::default()
				.with_include("*/launch/*")
				.with_include("*/launch.rs"),
			launch_file: WsPathBuf::new("launch.ron"),
			root_dir: WsPathBuf::default(),
			snippets_dir: WsPathBuf::new("target/snippets"),
			html_dir: WsPathBuf::new("target/client"),
			analytics_dir: WsPathBuf::new("target/analytics"),
			client_islands_path: WsPathBuf::new("target/client_islands.ron"),
		}
	}
}

impl WorkspaceConfig {
	/// Returns the snippets directory.
	pub fn snippets_dir(&self) -> &WsPathBuf { &self.snippets_dir }

	/// Creates a file path in the format of `path/to/file:line:col.rs`.
	///
	/// Uses [`Self::snippets_dir`] as the base.
	pub fn rsx_snippet_path(
		&self,
		path: impl AsRef<Path>,
		start_line: u32,
	) -> WsPathBuf {
		let mut path = path.as_ref().to_path_buf();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name = format!("{}:{}.rsx.ron", file_stem, start_line);
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	/// Creates a file path in the format of `path/to/file.ron`.
	///
	/// We need the index because some files may have multiple LangSnippets
	/// and we don't always have the span.
	/// Uses [`Self::snippets_dir`] as the base.
	pub fn lang_snippet_path(&self, path: &WsPathBuf, index: u64) -> WsPathBuf {
		let mut path = path.clone();
		let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
		let snippet_file_name = format!("{}-{}.lang.ron", file_stem, index);
		path.set_file_name(snippet_file_name);
		self.snippets_dir.join(path)
	}

	/// Returns `true` if the path passes the snippet filter.
	pub fn passes(&self, path: impl AsRef<Path>) -> bool {
		self.snippet_filter.passes(path.as_ref().to_string_lossy())
	}

	/// Returns all files in the root directory that pass the snippet filter.
	pub fn get_files(&self) -> Result<Vec<AbsPathBuf>, FsError> {
		ReadDir::files_recursive(&self.root_dir.into_abs())?
			.into_iter()
			.filter(|path| self.snippet_filter.passes(path.to_string_lossy()))
			.map(|path| AbsPathBuf::new(path))
			.collect()
	}
}
