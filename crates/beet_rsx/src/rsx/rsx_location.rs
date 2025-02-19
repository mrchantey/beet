use crate::prelude::*;
use rapidhash::rapidhash;

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
	/// in the cli its set via the file path,
	/// when setting this it must be in the same
	/// format as file!() would return
	pub file: String,
	pub filename_hash: u64,
	pub line: usize,
	pub col: usize,
}
impl Default for RsxMacroLocation {
	fn default() -> Self { Self::new("placeholder", 0, 0) }
}

impl RsxMacroLocation {
	pub fn new(file: impl Into<String>, line: usize, col: usize) -> Self {
		let file = file.into();
		let filename_hash = rapidhash(file.as_bytes());
		Self {
			file,
			filename_hash,
			line,
			col,
		}
	}
	pub fn file(&self) -> &str { &self.file }
	pub fn line(&self) -> usize { self.line }
	pub fn col(&self) -> usize { self.col }

	/// The only place it is allowed to hash a filename, its easy for 
	/// hashing implementations to drift and we depend on consistency.
	/// The only exception is [RstmlToRsxTemplate], which is an upstream 
	/// feature gated dependency, it uses the same techinque.
	pub fn hash_filename(filename: &str) -> u64 {
		rapidhash(filename.as_bytes())
	}

	pub fn new_global_idx(&self, idx: u32) -> GlobalRsxIdx {
		GlobalRsxIdx::new(&self.file, self.line as u32, self.col as u32, idx)
	}
}
