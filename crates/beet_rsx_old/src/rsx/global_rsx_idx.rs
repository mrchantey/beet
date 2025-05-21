#[cfg(feature = "bevy")]
use bevy::prelude::*;
use rapidhash::RapidHasher;
use std::hash::Hasher;

/// An RsxIdx is unique only to the macro the node was created in,
/// but for techniques like hot reloading we need to know not only
/// the local index but enough to distinguish it from nodes
/// in other trees.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
	feature = "bevy",
	derive(bevy::prelude::Component, bevy::prelude::Reflect)
)]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct GlobalRsxIdx {
	idx: u32,
	/// The actual [`RsxLocationHash`] is too expensive to store,
	/// it can be found at every [WebNode] so propagate it from there if needed.
	/// Rapidhash seed is consistent across macro and runtime hashing
	macro_location_hash: u64,
}

#[allow(deprecated)]
impl GlobalRsxIdx {
	pub fn idx(&self) -> u32 { self.idx }
	pub fn macro_location_hash(&self) -> u64 { self.macro_location_hash }
	pub fn new(macro_location_hash: u64, idx: u32) -> Self {
		Self {
			macro_location_hash,
			idx,
		}
	}

	pub fn into_hash(&self) -> u64 {
		let mut hasher = RapidHasher::default_const();
		hasher.write_u64(self.macro_location_hash);
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
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[test]
	#[ignore = "its unused and this test is a bit silly"]
	fn works() {
		let idx = GlobalRsxIdx::new(0, 123);
		let hash = idx.into_hash();
		expect(hash).not().to_be(0);
		expect(idx.into_hash_str().len()).to_be(8);
		// seemingly several reasons why the hash is different on wasm
		#[cfg(target_arch = "wasm32")]
		expect(idx.into_hash_str()).to_be("8G5IDfZS");
		#[cfg(not(target_arch = "wasm32"))]
		expect(idx.into_hash_str()).to_be("aVVe1woh");
	}
}
