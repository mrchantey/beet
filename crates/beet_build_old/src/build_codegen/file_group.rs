use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;
use sweet::prelude::*;

/// Definition for a group of files that should be collected together.
/// This is used as a field of types like [`ComponentFileGroup`] and [`RoutesFileGroup`].
#[derive(Debug, Default, PartialEq, Clone, Parser, Serialize, Deserialize)]
pub struct FileGroup {
	/// The directory where the files are located.
	#[arg(long, default_value = ".")]
	#[serde(rename = "path")]
	pub src: AbsPathBuf,
	/// Include and exclude filters for the files.
	#[command(flatten)]
	#[serde(flatten)]
	pub filter: GlobFilter,
}

impl FileGroup {
	pub fn new(src: AbsPathBuf) -> Self {
		Self {
			src,
			filter: GlobFilter::default(),
		}
	}

	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

	/// Perform a [`ReadDir`], returning all files in the directory
	/// relative this src
	pub fn collect_files(&self) -> Result<Vec<AbsPathBuf>> {
		let items = ReadDir {
			files: true,
			recursive: true,
			..Default::default()
		}
		.read(&self.src)?
		.into_iter()
		.filter_map(|path| {
			if self.filter.passes(&path) {
				// should be path+self.src?
				Some(AbsPathBuf::new(path))
			} else {
				None
			}
		})
		.collect::<Result<Vec<_>, FsError>>()?;
		Ok(items)
	}

	#[cfg(test)]
	pub fn test_site() -> Self {
		Self::new(
			WorkspacePathBuf::new("crates/beet_router/src/test_site")
				.into_abs()
				.unwrap(),
		)
	}
	#[cfg(test)]
	pub fn test_site_pages() -> Self {
		Self::new(
			WorkspacePathBuf::new("crates/beet_router/src/test_site/pages")
				.into_abs()
				.unwrap(),
		)
		.with_filter(
			GlobFilter::default()
				.with_include("*.rs")
				.with_exclude("*mod.rs"),
		)
	}
	#[cfg(test)]
	pub fn test_site_markdown() -> Self {
		Self::new(
			WorkspacePathBuf::new("crates/beet_router/src/test_site/test_docs")
				.into_abs()
				.unwrap(),
		)
		.with_filter(GlobFilter::default().with_include("*.md"))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(
			FileGroup::test_site()
				.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
				.collect_files()
				.unwrap()
				.len(),
		)
		.to_be(2);
	}
}
