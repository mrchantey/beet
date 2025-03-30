use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use sweet::prelude::*;

/// Definition for a group of files that should be collected together.
/// This is used as a field of types like [`ComponentFileGroup`] and [`RoutesFileGroup`].
#[derive(Debug, Default, Clone, Parser, Serialize, Deserialize)]
pub struct FileGroup {
	/// The directory where the files are located.
	#[arg(long, default_value = ".")]
	pub src: CanonicalPathBuf,
	/// Include and exclude filters for the files.
	#[command(flatten)]
	pub filter: GlobFilter,
}

impl FileGroup {
	pub fn new(src: CanonicalPathBuf) -> Self {
		Self {
			src,
			filter: GlobFilter::default(),
		}
	}

	pub fn new_workspace_rel(src: impl Into<WorkspacePathBuf>) -> Result<Self> {
		Ok(Self {
			src: src.into().into_canonical()?,
			filter: GlobFilter::default(),
		})
	}

	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

	pub fn collect_files(&self) -> Result<Vec<PathBuf>> {
		let items = ReadDir {
			files: true,
			recursive: true,
			..Default::default()
		}
		.read(&self.src)?
		.into_iter()
		.filter_map(|path| {
			if self.filter.passes(&path) {
				Some(path)
			} else {
				None
			}
		})
		.collect::<Vec<_>>();
		Ok(items)
	}

	#[cfg(test)]
	pub fn test_site_routes() -> Self {
		Self::new_workspace_rel("crates/beet_router/src/test_site/routes")
			.unwrap()
			.with_filter(
				GlobFilter::default()
					.with_include("*.rs")
					.with_exclude("*mod.rs"),
			)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(
			FileGroup::new_workspace_rel("crates/beet_router/src/test_site")
				.unwrap()
				.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
				.collect_files()
				.unwrap()
				.len(),
		)
		.to_be(2);
	}
}
