use super::FsError;
use super::FsResult;
use std::fs;
use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;


/// A nicer read file that actually outputs the missing path
pub struct ReadFile;

impl ReadFile {
	pub fn to_string(path: impl AsRef<Path>) -> FsResult<String> {
		std::fs::read_to_string(&path).map_err(|e| FsError::io(path, e))
	}
	pub fn to_bytes(path: impl AsRef<Path>) -> FsResult<Vec<u8>> {
		std::fs::read(&path).map_err(|e| FsError::io(path, e))
	}


	pub fn hash_file(path: impl AsRef<Path>) -> FsResult<u64> {
		let bytes = Self::to_bytes(path)?;
		let hash = Self::hash_bytes(&bytes);
		Ok(hash)
	}

	pub fn hash_bytes(bytes: &[u8]) -> u64 {
		let mut hasher = DefaultHasher::new();
		bytes.hash(&mut hasher);
		hasher.finish()
	}
	pub fn hash_string(str: &str) -> u64 {
		let bytes = str.as_bytes();
		Self::hash_bytes(bytes)
	}

	pub fn exists(path: impl AsRef<Path>) -> bool {
		match fs::exists(path) {
			Ok(true) => true,
			_ => false,
		}
	}
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;

	#[test]
	fn to_string() {
		let content =
			ReadFile::to_string(FsExt::test_dir().join("mod.rs")).unwrap();
		assert!(content.contains("pub mod included_dir;"));

		assert!(ReadFile::to_string(FsExt::test_dir().join("foo.rs")).is_err());
	}

	#[test]
	fn to_bytes() {
		let bytes =
			ReadFile::to_bytes(FsExt::test_dir().join("mod.rs")).unwrap();
		assert!(bytes.len() > 10);

		assert!(ReadFile::to_bytes(FsExt::test_dir().join("foo.rs")).is_err());
	}

	#[test]
	fn hash() {
		let hash1 =
			ReadFile::hash_file(FsExt::test_dir().join("mod.rs")).unwrap();
		let hash2 =
			ReadFile::hash_file(FsExt::test_dir().join("included_file.rs"))
				.unwrap();
		assert_ne!(hash1, hash2);

		let str =
			ReadFile::to_string(FsExt::test_dir().join("mod.rs")).unwrap();
		let hash3 = ReadFile::hash_string(&str);
		assert_eq!(hash3, hash1);
	}
}
