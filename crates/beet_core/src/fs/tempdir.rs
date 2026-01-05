use std::path::Path;
use std::path::PathBuf;

use crate::prelude::*;
use uuid::Uuid;

/// A temporary directory that is automatically deleted when dropped.
///
/// This struct provides a safe way to create temporary directories that are
/// guaranteed to be cleaned up when they go out of scope. The directory name
/// is generated using a UUID v4 to ensure uniqueness and avoid collisions.
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
	path: AbsPathBuf,
	/// Do not remove the directory on drop
	keep: bool,
}

impl AsRef<Path> for TempDir {
	fn as_ref(&self) -> &Path { &self.path }
}

impl std::ops::Deref for TempDir {
	type Target = AbsPathBuf;
	fn deref(&self) -> &Self::Target { &self.path }
}

impl Default for TempDir {
	fn default() -> Self { Self::new().unwrap() }
}

impl TempDir {
	/// Creates a new temporary directory in the system's temporary directory.
	///
	/// The directory is created with a unique name in the format `beet_tmp_<uuid>`,
	/// where `<uuid>` is a randomly generated UUID v4. The directory will be
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
		let dir_name = format!("beet_tmp_{}", Uuid::new_v4().to_string());
		let dir_path = temp_dir.join(dir_name);
		Self::new_with_path(dir_path)
	}

	/// Creates a new temporary directory relative to the workspace root.
	///
	/// The directory is created at `<workspace_root>/target/tmp/beet_tmp_<uuid>`,
	/// where `<uuid>` is a randomly generated UUID v4. This is useful for keeping
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
		let dir_name =
			format!("target/tmp/beet_tmp_{}", Uuid::new_v4().to_string());
		let dir_path = workspace_root.join(dir_name);
		Self::new_with_path(dir_path)
	}

	fn new_with_path(path: PathBuf) -> FsResult<Self> {
		if path.exists() {
			return Err(FsError::AlreadyExists { path });
		}

		fs_ext::create_dir_all(&path)?;
		Ok(Self {
			path: AbsPathBuf::new(path)?,
			keep: false,
		})
	}

	/// Returns the path to the temporary directory.
	pub fn path(&self) -> &AbsPathBuf { &self.path }

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
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn test_tempdir_new_creates_and_cleans_up() {
		let dir_path;
		{
			// Create a temp directory
			let temp = TempDir::new().expect("Failed to create temp directory");
			dir_path = temp.path.clone();

			// Verify it exists
			dir_path.exists().xpect_true();
			dir_path.is_dir().xpect_true();
		} // temp is dropped here

		// Verify it was cleaned up
		dir_path.exists().xpect_false();
	}

	#[test]
	fn test_tempdir_workspace_relative_creates_and_cleans_up() {
		let dir_path;
		{
			// Create a workspace-relative temp directory
			let temp = TempDir::new_workspace()
				.expect("Failed to create workspace-relative temp directory");
			dir_path = temp.path.clone();

			// Verify it exists
			dir_path.exists().xpect_true();
			dir_path.is_dir().xpect_true();

			// Verify it's in the workspace
			let path_str = dir_path.to_string();
			path_str.contains("target/tmp/beet_tmp_").xpect_true();
		} // temp is dropped here

		// Verify it was cleaned up
		dir_path.exists().xpect_false();
	}

	#[test]
	fn test_tempdir_unique_names() {
		// Create multiple temp directories and verify they have unique names
		let temp1 =
			TempDir::new().expect("Failed to create first temp directory");
		let temp2 =
			TempDir::new().expect("Failed to create second temp directory");

		temp1.path().clone().xpect_not_eq(temp2.path().clone());
	}

	#[test]
	fn test_tempdir_keep_prevents_cleanup() {
		let dir_path;
		{
			// Create a temp directory and mark it to keep
			let temp = TempDir::new()
				.expect("Failed to create temp directory")
				.keep();
			dir_path = temp.path().clone();

			// Verify it exists
			dir_path.exists().xpect_true();
		} // temp is dropped here

		// Verify it was NOT cleaned up because we called keep()
		dir_path.exists().xpect_true();

		// Manual cleanup for this test
		fs_ext::remove(&dir_path).ok();
	}
}
