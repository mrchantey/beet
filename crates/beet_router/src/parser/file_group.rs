use clap::Parser;
use sweet::prelude::GlobFilter;
use sweet::prelude::WorkspacePathBuf;

/// Definition for a group of files that should be collected together.
/// This is used as a field of types like [`ComponentFileGroup`] and [`RoutesFileGroup`].
#[derive(Debug, Clone, Parser, serde::Serialize, serde::Deserialize)]
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

	#[test]
	fn works() { let _group: FileGroup = "foobar".into(); }
}
