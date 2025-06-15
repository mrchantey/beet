use crate::prelude::*;
use anyhow::Result;
use beet_common::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;


/// Config included in the `beet.toml` file for a file group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Sendit)]
#[sendit(derive(Component))]
pub struct FileGroupConfig {
	#[serde(flatten)]
	pub file_group: FileGroup,
	#[serde(flatten)]
	pub codegen: CodegenFile,
	#[serde(flatten)]
	pub modifier: ModifyRouteFileMethod,
}


impl FileGroupConfig {
	pub fn into_bundle(self) -> impl Bundle {
		(self.file_group, self.codegen.sendit(), self.modifier)
	}
}

/// Definition for a group of files that should be collected together.
/// This is used as a field of types like [`ComponentFileGroup`] and [`RoutesFileGroup`].
#[derive(
	Debug, Default, PartialEq, Clone, Parser, Serialize, Deserialize, Component,
)]
#[require(CodegenFileSendit)]
pub struct FileGroup {
	/// Setting this will include the file group in the route tree,
	/// defaults to `false`.
	pub is_pages: bool,
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
			is_pages: false,
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
			WsPathBuf::new("crates/beet_router/src/test_site")
				.into_abs(),
		)
	}
	#[cfg(test)]
	pub fn test_site_pages() -> Self {
		Self::new(
			WsPathBuf::new("crates/beet_router/src/test_site/pages")
				.into_abs(),
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
			WsPathBuf::new("crates/beet_router/src/test_site/test_docs")
				.into_abs(),
		)
		.with_filter(GlobFilter::default().with_include("*.md"))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::GlobFilter;
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
