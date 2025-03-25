use crate::prelude::*;
use std::path::PathBuf;
use sweet::prelude::GlobFilter;

/// File groups are collections of files that should be collected together,
/// the most common example being a [`TreeFileGroup`] which creates routes
/// for each file in a directory.
///
/// hese config files are simply passed to the cli which handles the parsing
/// and code gen.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FileGroupConfig {
	pub app_cx: AppContext,
	pub groups: Vec<FileGroup>,
}

impl FileGroupConfig {
	/// Create a new Collection Builder.
	/// ## Panics
	/// Panics if the current working directory cannot be determined.
	pub fn new(app_cx: AppContext) -> Self {
		Self {
			app_cx,
			groups: Vec::new(),
		}
	}

	pub fn add_group(mut self, group: impl Into<FileGroup>) -> Self {
		self.groups.push(group.into());
		self
	}

	/// Serializes self and writes to stdout, which is collected by the beet cli.
	///
	/// ## Panics
	/// Panics if serialization fails.
	pub fn export(&self) {
		let ron = ron::ser::to_string_pretty(self, Default::default())
			.expect("failed to serialize");
		println!("{}", ron);
	}
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum FileGroup {
	/// Config for an additional [`FileGroupConfig`] that should also be exported.
	Child(FileGroupConfig),
	/// Config for a [`GlobFileGroup`].
	Glob(GlobFileGroup),
	/// Config for a [`TreeFileGroup`].
	Tree(TreeFileGroup),
}

impl Into<FileGroup> for FileGroupConfig {
	fn into(self) -> FileGroup { FileGroup::Child(self) }
}
impl Into<FileGroup> for GlobFileGroup {
	fn into(self) -> FileGroup { FileGroup::Glob(self) }
}
impl Into<FileGroup> for TreeFileGroup {
	fn into(self) -> FileGroup { FileGroup::Tree(self) }
}


#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GlobFileGroup {
	/// The directory relative to the [`FileGroupConfig::root_dir`] where the files are located.
	pub src_dir: PathBuf,
	/// The directory relative to the [`FileGroupConfig::root_dir`] to build the collected items.
	pub dst_file: PathBuf,
	pub filter: GlobFilter,
}

impl GlobFileGroup {
	pub fn new(
		src_dir: impl Into<PathBuf>,
		dst_file: impl Into<PathBuf>,
		filter: GlobFilter,
	) -> Self {
		Self {
			src_dir: src_dir.into(),
			dst_file: dst_file.into(),
			filter,
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;
	use beet_rsx::as_beet::*;


	#[test]
	fn works() {
		let _builder = FileGroupConfig::new(app_cx!())
			.add_group(FileGroupConfig::new(app_cx!()))
			.add_group(GlobFileGroup::new(
				".",
				"my_group.rs",
				GlobFilter::default(),
			))
			.add_group(TreeFileGroup::new("routes"));
		//.export();
	}
}
