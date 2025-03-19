use std::path::PathBuf;
use sweet::prelude::GlobFilter;




#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FileGroupConfig {
	pub cwd: PathBuf,
	pub groups: Vec<FileGroup>,
}

impl FileGroupConfig {
	/// Create a new Collection Builder.
	/// ## Panics
	/// Panics if the current working directory cannot be determined.
	pub fn new() -> Self {
		Self {
			cwd: std::env::current_dir().unwrap(),
			groups: Vec::new(),
		}
	}

	pub fn add_group(&mut self, group: impl Into<FileGroup>) {
		self.groups.push(group.into());
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
	Glob(GlobFileGroup),
	Tree(TreeFileGroup),
}

impl Into<FileGroup> for GlobFileGroup {
	fn into(self) -> FileGroup { FileGroup::Glob(self) }
}
impl Into<FileGroup> for TreeFileGroup {
	fn into(self) -> FileGroup { FileGroup::Tree(self) }
}


#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GlobFileGroup {
	pub src_dir: PathBuf,
	/// The file to be built containing the collected items.
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


#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TreeFileGroup {
	pub src_dir: PathBuf,
}

impl Default for TreeFileGroup {
	fn default() -> Self {
		Self {
			src_dir: PathBuf::from("routes"),
		}
	}
}

impl TreeFileGroup {
	pub fn new(dir: impl Into<PathBuf>) -> Self {
		Self {
			src_dir: dir.into(),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let _builder = FileGroupConfig::new();
		// expect(true).to_be_false();
	}
}
