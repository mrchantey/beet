use rapidhash::RapidHasher;
use rapidhash::rapidhash;
use std::hash::Hash;
use std::hash::Hasher;
use sweet::prelude::*;

/// File location of the first symbol inside an rsx macro, used by [RsxTemplate]
/// to reconcile rsx nodes with html partials
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
	pub line: usize,
	pub col: usize,
}
// useful for example when converting strings to an RsxRoot,
// where they can safely have an invalid file.
impl Default for RsxMacroLocation {
	fn default() -> Self { Self::new(WorkspacePathBuf::new(file!()), 0, 0) }
}

impl std::fmt::Display for RsxMacroLocation {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{}", self.file.display(), self.line, self.col)
	}
}

impl RsxMacroLocation {
	pub fn new(file: WorkspacePathBuf, line: usize, col: usize) -> Self {
		Self { file, line, col }
	}
	pub fn file(&self) -> &WorkspacePathBuf { &self.file }
	pub fn line(&self) -> usize { self.line }
	pub fn col(&self) -> usize { self.col }

	pub fn into_hash(&self) -> u64 {
		let mut hasher = RapidHasher::default_const();
		self.file.hash(&mut hasher);
		hasher.write_usize(self.line);
		hasher.write_usize(self.col);
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
