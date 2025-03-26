use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use sweet::prelude::*;

/// Definition for a group of files that should be collected together.
/// This is used as a field of types like [`ComponentFileGroup`] and [`RoutesFileGroup`].
#[derive(
	Debug, Default, Clone, Parser, serde::Serialize, serde::Deserialize,
)]
pub struct FileGroup {
	/// The directory where the files are located.
	#[arg(long, default_value = ".")]
	pub src: WorkspacePathBuf,
	/// Include and exclude filters for the files.
	#[command(flatten)]
	pub filter: GlobFilter,
}

impl FileGroup {
	pub fn new(src: impl Into<WorkspacePathBuf>) -> Self {
		Self {
			src: src.into(),
			filter: GlobFilter::default(),
		}
	}
	pub fn new_with_filter(
		src: impl Into<WorkspacePathBuf>,
		filter: GlobFilter,
	) -> Self {
		Self {
			src: src.into(),
			filter,
		}
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
		.read(self.src.into_canonical()?)?
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
}

impl Into<FileGroup> for &str {
	fn into(self) -> FileGroup {
		FileGroup {
			src: WorkspacePathBuf::new(self),
			filter: GlobFilter::default(),
		}
	}
}
impl Into<FileGroup> for WorkspacePathBuf {
	fn into(self) -> FileGroup {
		FileGroup {
			src: self,
			filter: GlobFilter::default(),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(
			FileGroup::new("crates/beet_router/src/test_site")
				.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
				.collect_files()
				.unwrap()
				.len(),
		)
		.to_be(2);
	}
}
