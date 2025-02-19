use super::RsxMacroLocation;
use rapidhash::RapidHasher;
use std::hash::Hasher;

/// Unique identifier for every node in an rsx tree,
/// and assigned to html elements that need it.
/// The value is incremented every time an rsx node is encountered
/// in a dfs pattern like [RsxVisitor].
pub type RsxIdx = u32;

/// An RsxIdx is unique only to the macro the node was created in,
/// but for techniques like hot reloading we need to know not only
/// the local index but enough to distinguish it from nodes
/// in other trees.
#[deprecated = "not in use"]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GlobalRsxIdx {
	idx: RsxIdx,
	/// The actual filename string is too expensive to store,
	/// it can be found at every [RsxRoot] so propagate it from there if needed.
	/// Rapidhash seed is consistent across macro and runtime hashing
	filename_hash: u64,
	line: u32,
	col: u32,
}

#[allow(deprecated)]
impl GlobalRsxIdx {
	pub fn filename_hash(&self) -> u64 { self.filename_hash }
	pub fn line(&self) -> u32 { self.line }
	pub fn col(&self) -> u32 { self.col }
	pub fn idx(&self) -> RsxIdx { self.idx }
	pub fn new(filename: &str, line: u32, col: u32, idx: RsxIdx) -> Self {
		Self {
			filename_hash: RsxMacroLocation::hash_filename(filename),
			line,
			col,
			idx,
		}
	}

	pub fn into_hash(&self) -> u64 {
		let mut hasher = RapidHasher::default_const();
		hasher.write_u64(self.filename_hash);
		hasher.write_u32(self.line);
		hasher.write_u32(self.col);
		hasher.write_u32(self.idx);
		hasher.finish()
	}
	/// an 8 char hexadecimal string for use in html attributes
	pub fn into_hash_str(&self) -> String {
		hash_to_alphanumeric(self.into_hash(), 8)
	}
}

fn hash_to_alphanumeric(hash: u64, length: usize) -> String {
	const CHARSET: &[u8] =
		b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
	const BASE: u64 = 62; // Length of CHARSET

	let mut result = Vec::new();
	let mut n = hash;

	while n > 0 {
		let idx = (n % BASE) as usize;
		result.push(CHARSET[idx] as char);
		n /= BASE;
	}

	// Pad with '0' if needed
	while result.len() < length {
		result.push('0');
	}

	// Reverse because we built it backwards
	result.reverse();

	// Truncate to desired length
	result.truncate(length);

	result.into_iter().collect()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	#[allow(deprecated)]
	fn works() {
		let idx = GlobalRsxIdx::new("file", 1, 2, 3);
		let hash = idx.into_hash();
		expect(hash).not().to_be(0);
		expect(idx.into_hash_str().len()).to_be(8);
		expect(idx.into_hash_str()).to_be("6cdNt13a");
	}
}
