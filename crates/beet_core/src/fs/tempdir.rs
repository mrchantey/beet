use std::path::Path;
use std::path::PathBuf;

use crate::prelude::*;

/// A temporary directory that is automatically deleted when dropped.
///
/// This struct provides a safe way to create temporary directories that are
/// guaranteed to be cleaned up when they go out of scope. The directory name
/// is generated using a UUID v7 to ensure uniqueness and avoid collisions.
///
/// This type is not [`Clone`] as it removes the underlying directory on drop.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
///
/// // Create a temporary directory in the system temp folder
/// let temp = TempDir::new().unwrap();
/// ```
#[derive(Debug)]
pub struct TempDir {
	/// The path to the temporary directory
	path: AbsPath,
	/// Do not remove the directory on drop
	keep: bool,
}

impl AsRef<Path> for TempDir {
	fn as_ref(&self) -> &Path { self.path.as_ref() }
}

impl std::ops::Deref for TempDir {
	type Target = AbsPath;
	fn deref(&self) -> &Self::Target { &self.path }
}

impl Default for TempDir {
	fn default() -> Self { Self::new().unwrap() }
}

impl TempDir {
	/// Creates a new temporary directory in the system's temporary directory.
	///
	/// The directory is created with a unique name in the format `beet_tmp_<uuid>`,
	/// where `<uuid>` is a randomly generated UUID v7. The directory will be
	/// automatically deleted when the `TempDir` instance is dropped.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	///
	/// let temp = TempDir::new().unwrap();
	/// ```
	pub fn new() -> FsResult<Self> {
		let temp_dir = std::env::temp_dir();
		let dir_name = format!("beet_tmp_{}", uuid_ext::now_v7());
		let dir_path = temp_dir.join(dir_name);
		Self::new_with_path(dir_path)
	}

	/// Create a new temporary directory in `target/tmp`, relative to the workspace root.
	/// The `CARGO_TARGET_DIR` env var is not used.
	pub fn new_ws() -> FsResult<Self> {
		let workspace_root = fs_ext::workspace_root();
		let dir_name = format!("target/tmp/beet_tmp_{}", uuid_ext::now_v7());
		let dir_path = workspace_root.join(dir_name);
		Self::new_with_path(dir_path)
	}

	/// Creates a new temporary directory relative to the workspace root.
	///
	/// The directory is created at `<workspace_root>/target/tmp/beet_tmp_<uuid>`,
	/// where `<uuid>` is a randomly generated UUID v7. This is useful for keeping
	/// temporary files within the project structure.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	///
	/// let temp = TempDir::new_workspace().unwrap();
	/// ```
	pub fn new_workspace() -> FsResult<Self> {
		let workspace_root = fs_ext::workspace_root();
		let dir_name = format!("target/tmp/beet_tmp_{}", uuid_ext::now_v7());
		let dir_path = workspace_root.join(dir_name);
		Self::new_with_path(dir_path)
	}

	fn new_with_path(path: PathBuf) -> FsResult<Self> {
		if path.exists() {
			return Err(FsError::AlreadyExists { path });
		}

		fs_ext::create_dir_all(&path)?;
		Ok(Self {
			path: AbsPath::new(path)?,
			keep: false,
		})
	}

	/// Returns the path to the temporary directory.
	pub fn path(&self) -> &AbsPath { &self.path }

	/// Marks this temporary directory to be kept on drop.
	///
	/// By default, the directory is removed when the `TempDir` is dropped.
	/// Calling this method prevents the automatic cleanup, leaving the directory
	/// on the filesystem.
	pub fn keep(mut self) -> Self {
		self.keep = true;
		self
	}
}

impl Drop for TempDir {
	/// Automatically removes the temporary directory when the `TempDir` goes out of scope.
	///
	/// Any errors during removal are silently ignored to prevent panics during drop.
	/// If `keep()` was called, the directory will not be removed.
	fn drop(&mut self) {
		if !self.keep {
			fs_ext::remove(&self.path).ok();
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;

	#[crate::test]
	fn test_tempdir_new_creates_and_cleans_up() {
		let dir_path;
		{
			// Create a temp directory
			let temp = TempDir::new().expect("Failed to create temp directory");
			dir_path = temp.path.clone();

			// Verify it exists
			fs_ext::exists(&dir_path).unwrap().xpect_true();
			fs_ext::is_dir(&dir_path).xpect_true();
		} // temp is dropped here

		// Verify it was cleaned up
		fs_ext::exists(&dir_path).unwrap().xpect_false();
	}

	#[crate::test]
	fn test_tempdir_workspace_relative_creates_and_cleans_up() {
		let dir_path;
		{
			// Create a workspace-relative temp directory
			let temp = TempDir::new_workspace()
				.expect("Failed to create workspace-relative temp directory");
			dir_path = temp.path.clone();

			// Verify it exists
			fs_ext::exists(&dir_path).unwrap().xpect_true();
			fs_ext::is_dir(&dir_path).xpect_true();

			// Verify it's in the workspace
			dir_path.contains("target/tmp/beet_tmp_").xpect_true();
		} // temp is dropped here

		// Verify it was cleaned up
		fs_ext::exists(&dir_path).unwrap().xpect_false();
	}

	#[crate::test]
	fn test_tempdir_unique_names() {
		// Create multiple temp directories and verify they have unique names
		let temp1 =
			TempDir::new().expect("Failed to create first temp directory");
		let temp2 =
			TempDir::new().expect("Failed to create second temp directory");

		temp1.path().clone().xpect_not_eq(temp2.path().clone());
	}

	#[crate::test]
	fn test_tempdir_keep_prevents_cleanup() {
		let dir_path;
		{
			// Create a temp directory and mark it to keep
			let temp = TempDir::new()
				.expect("Failed to create temp directory")
				.keep();
			dir_path = temp.path().clone();

			// Verify it exists
			fs_ext::exists(&dir_path).unwrap().xpect_true();
		} // temp is dropped here

		// Verify it was NOT cleaned up because we called keep()
		fs_ext::exists(&dir_path).unwrap().xpect_true();

		// Manual cleanup for this test
		fs_ext::remove(&dir_path).ok();
	}
}
