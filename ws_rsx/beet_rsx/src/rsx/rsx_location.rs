use rapidhash::RapidHasher;
use rapidhash::rapidhash;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use sweet::prelude::*;

/// File location of the first symbol inside an rsx macro, used by [RsxTemplate]
/// to reconcile web nodes with templates
///
/// ```rust ignore
/// # use beet_rsx_macros::rsx;
/// let tree = rsx!{<div>hello</div>};
/// //              ^ this location
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxMacroLocation {
	/// Workspace relative path to the file, its essential to use consistent paths
	/// as this struct is created in several places from all kinds concatenations,
	/// and we need PartialEq & Hash to be identical.
	pub file: WorkspacePathBuf,
	/// The 1 indexed line in the source file, reflecting the behavior of `line!()`
	pub line: u32,
	/// The 0 indexed column in the source file, reflecting the behavior of `column!()`
	pub col: u32,
}

impl std::fmt::Display for RsxMacroLocation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{}", self.file.display(), self.line, self.col)
	}
}

impl RsxMacroLocation {
	pub fn placeholder() -> Self {
		Self {
			file: WorkspacePathBuf::default(),
			line: 0,
			col: 0,
		}
	}
	/// Create a new [RsxMacroLocation] from a file path where it should represent
	/// the entire file, the line and column are set to 1 and 0 respectively.
	pub fn new_for_file(file: impl AsRef<Path>) -> Self {
		Self {
			file: WorkspacePathBuf::new(file),
			line: 1,
			col: 0,
		}
	}

	/// Create a new [RsxMacroLocation] from a file path, line and column,
	/// most commonly used by the `rsx!` macro.
	/// ## Example
	///
	/// ```rust
	/// # use beet_rsx::as_beet::*;
	/// let loc = RsxMacroLocation::new(file!(), line!(), column!());
	/// let loc = rsx!{}.location().unwrap();
	pub fn new(
		workspace_file_path: impl AsRef<Path>,
		line: u32,
		col: u32,
	) -> Self {
		Self {
			file: WorkspacePathBuf::new(workspace_file_path),
			line,
			col,
		}
	}
	pub fn file(&self) -> &WorkspacePathBuf { &self.file }
	pub fn line(&self) -> u32 { self.line }
	pub fn col(&self) -> u32 { self.col }

	pub fn into_hash(&self) -> u64 {
		let mut hasher = RapidHasher::default_const();
		self.file.hash(&mut hasher);
		hasher.write_u32(self.line);
		hasher.write_u32(self.col);
		hasher.finish()
	}

	/// The only place it is allowed to hash a filename, its easy for
	/// hashing implementations to drift and we depend on consistency.
	/// The only exception is [RstmlToRsxTemplate], which is an upstream
	/// feature gated dependency, it uses the same techinque.
	pub fn hash_filename(filename: &str) -> u64 {
		rapidhash(filename.as_bytes())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let buf1 = WorkspacePathBuf::new("foo");
		let buf2 = WorkspacePathBuf::new("bar");
		let loc1 = RsxMacroLocation::new(buf1.clone(), 0, 0);
		let loc2 = RsxMacroLocation::new(buf2.clone(), 0, 0);
		expect(loc1.into_hash()).not().to_be(loc2.into_hash());
		let loc1 = RsxMacroLocation::new(buf1.clone(), 0, 0);
		let loc2 = RsxMacroLocation::new(buf1.clone(), 1, 1);
		expect(loc1.into_hash()).not().to_be(loc2.into_hash());
	}
}
