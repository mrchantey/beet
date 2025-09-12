use crate::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// The workspace relative directory for this file,
/// internally using the `file!()` macro.
/// ## Example
///
/// ```rust
/// # use beet_utils::prelude::*;
/// let dir = dir!();
/// ```
#[macro_export]
macro_rules! dir {
	() => {
		std::path::Path::new(file!()).parent().unwrap()
	};
}


/// Better Fs, actually outputs missing path
pub struct FsExt;

impl FsExt {
	pub fn current_dir() -> FsResult<PathBuf> {
		std::env::current_dir().map_err(|e| FsError::io(".", e))
	}

	/// Copy a directory recursively, creating it if it doesnt exist
	/// This also provides consistent behavior with the `cp` command:
	/// -
	pub fn copy_recursive(
		source: impl AsRef<Path>,
		destination: impl AsRef<Path>,
	) -> FsResult {
		let source = source.as_ref();
		let destination = destination.as_ref();

		fs::create_dir_all(&destination).ok();
		for entry in ReadDir::all(source)? {
			let file_name = PathExt::file_name(&entry)?;
			if entry.is_dir() {
				Self::copy_recursive(&entry, destination.join(file_name))?;
			} else {
				fs::copy(&entry, destination.join(file_name))
					.map_err(|err| FsError::io(entry, err))?;
			}
		}
		Ok(())
	}

	/// remove a directory and all its contents
	pub fn remove(path: impl AsRef<Path>) -> FsResult {
		let path = path.as_ref();
		fs::remove_dir_all(path).map_err(|err| FsError::io(path, err))?;
		Ok(())
	}

	/// Async: remove a directory and all its contents
	#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
	pub async fn remove_async(path: impl AsRef<Path>) -> FsResult {
		let path = path.as_ref();
		tokio::fs::remove_dir_all(path)
			.await
			.map_err(|err| FsError::io(path, err))?;
		Ok(())
	}


	// pub fn dir_contains(path: PathBuf, pattern: &str) -> bool {
	// 	let pattern = Pattern::new(pattern).unwrap();
	// 	glob::glob_with(
	// 		&pattern.to_string(),
	// 		glob::MatchOptions {
	// 			case_sensitive: false,
	// 			require_literal_separator: false,
	// 			require_literal_leading_dot: false,
	// 		},
	// 	)
	// 	read_dir_recursive(path)
	// 		.iter()
	// 		.any(|p| pattern. p.to_str().unwrap().contains(pattern))
	// }


	/// 1. tries to get the `SWEET_ROOT` env var.
	/// 2. if wasm, returns an empty path
	/// 3. Otherwise return the closest ancestor (inclusive) that contains a `Cargo.lock` file
	/// 4. Otherwise returns cwd
	///
	/// ## Panics
	/// - The current directory is not found
	/// - Insufficient permissions to access the current directory
	pub fn workspace_root() -> PathBuf { crate::prelude::workspace_root() }


	/// Write a file, ensuring the path exists
	pub fn write(path: impl AsRef<Path>, data: impl AsRef<[u8]>) -> FsResult {
		let path = path.as_ref();
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)
				.map_err(|err| FsError::io(parent, err))?;
		}
		fs::write(path, data).map_err(|err| FsError::io(path, err))?;
		Ok(())
	}

	/// Async version of write: Write a file, ensuring the path exists
	#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
	pub async fn write_async(
		path: impl AsRef<Path>,
		data: impl AsRef<[u8]>,
	) -> FsResult {
		use tokio::fs;
		let path = path.as_ref();
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)
				.await
				.map_err(|err| FsError::io(parent, err))?;
		}
		fs::write(path, data)
			.await
			.map_err(|err| FsError::io(path, err))?;
		Ok(())
	}

	/// Write a file only if the data is different from the existing file,
	/// if the file does not exist, it will be created.
	pub fn write_if_diff(
		path: impl AsRef<Path>,
		data: impl AsRef<[u8]>,
	) -> FsResult {
		let path = path.as_ref();
		match fs::read(path) {
			Ok(existing_data) if existing_data == data.as_ref() => {
				return Ok(());
			}
			_ => {
				Self::write(path, data)?;
			}
		}
		Ok(())
	}
}

#[cfg(test)]
impl FsExt {
	pub fn test_dir() -> PathBuf {
		Self::workspace_root()
			.join(Path::new("crates/beet_utils/src/fs/test_dir"))
	}
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use super::FsExt;

	#[test]
	fn workspace_root() {
		assert_eq!(
			FsExt::workspace_root()
				.file_stem()
				.unwrap()
				.to_str()
				.unwrap(),
			"beet"
		);
		assert!(FsExt::workspace_root().join("Cargo.lock").exists());
	}
}
