use super::error::Error;
use super::error::Result;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

/// Config for the template creation stage of the build process
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Resource)]
pub struct BuildTemplatesConfig {
	/// Filter for files that should be parsed,
	/// excludes 'target' and 'node_modules' directories by default
	filter: GlobFilter,
	/// The root directory for files including templates
	root_dir: WorkspacePathBuf,
}

impl Default for BuildTemplatesConfig {
	fn default() -> Self {
		Self {
			filter: GlobFilter::default()
				.with_exclude("*/target/*")
				.with_exclude("*/node_modules/*"),
			#[cfg(test)]
			root_dir: WorkspacePathBuf::new("crates/beet_router/src/test_site"),
			#[cfg(not(test))]
			root_dir: WorkspacePathBuf::default(),
		}
	}
}


impl BuildTemplatesConfig {

	pub fn get_files(&self) -> Result<Vec<WorkspacePathBuf>> {
		ReadDir::files_recursive(
			&self.root_dir.into_abs().map_err(Error::File)?,
		)
		.map_err(Error::File)?
		.into_iter()
		.filter(|path| self.filter.passes(path))
		.map(|path| {
			WorkspacePathBuf::new_from_cwd_rel(path).map_err(Error::File)
		})
		.collect::<Result<Vec<_>>>()
	}
}
