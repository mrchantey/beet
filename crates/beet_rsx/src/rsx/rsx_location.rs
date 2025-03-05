use rapidhash::rapidhash;
use rapidhash::RapidHasher;
use std::hash::Hasher;

/// File location of an rsx macro, used by [RsxTemplate]
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
	/// in the macro this is set via file!(),
	/// in the cli its set via the file path.
	/// When setting this it must be in the same
	/// format as file!() would return, but with forward slashes.
	/// We must use forward slashes because sometimes a wasm build will be used
	/// in combination with a windows build, and the paths must match.
	pub file: String,
	pub line: usize,
	pub col: usize,
}
impl Default for RsxMacroLocation {
	fn default() -> Self { Self::new("placeholder", 0, 0) }
}

impl RsxMacroLocation {
	pub fn new(file: impl Into<String>, line: usize, col: usize) -> Self {
		let file = file.into().replace("\\", "/");
		Self { file, line, col }
	}
	pub fn file(&self) -> &str { &self.file }
	pub fn line(&self) -> usize { self.line }
	pub fn col(&self) -> usize { self.col }

	pub fn into_hash(&self) -> u64 {
		let mut hasher = RapidHasher::default_const();
		hasher.write(self.file.as_bytes());
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
