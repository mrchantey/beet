use std::path::PathBuf;

use crate::prelude::*;
use uuid::Uuid;

/// A temporary directory that is automatically deleted when dropped.
///
/// This struct provides a safe way to create temporary directories that are
/// guaranteed to be cleaned up when they go out of scope. The directory name
/// is generated using a UUID v4 to ensure uniqueness and avoid collisions.
///
/// # Examples
///
/// ```no_run
/// use beet_utils::path_utils::TempDir;
///
/// // Create a temporary directory in the system temp folder
/// let temp = TempDir::new()?;
///
/// // Use the directory...
/// // It will be automatically deleted when `temp` goes out of scope
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct TempDir {
	path: AbsPathBuf,
	/// Do not remove the directory on drop
	keep: bool,
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
	/// The directory is created with a unique name in the format `beet_tmp<uuid>`,
	/// where `<uuid>` is a randomly generated UUID v4. The directory will be
	/// automatically deleted when the `TempDir` instance is dropped.
	///
	/// # Returns
	///
	/// Returns `Ok(TempDir)` if the directory was successfully created, or an
	/// error if directory creation failed.
	///
	/// # Examples
	///
	/// ```no_run
	/// use beet_utils::path_utils::TempDir;
	///
	/// let temp = TempDir::new()?;
	/// // Directory exists and can be used
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	pub fn new() -> FsResult<Self> {
		let temp_dir = std::env::temp_dir();
		let dir_name = format!("beet_tmp{}", Uuid::new_v4().to_string());
		let dir_path = temp_dir.join(dir_name);
		Self::new_with_path(dir_path)
	}

	/// Returns the path to the temporary directory.
	///
	/// # Examples
	///
	/// ```no_run
	/// use beet_utils::path_utils::TempDir;
	///
	/// let temp = TempDir::new()?;
	/// let path = temp.path();
	/// // Use the path to create files, etc.
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	pub fn path(&self) -> &AbsPathBuf { &self.path }

	/// Creates a new temporary directory relative to the workspace root.
	///
	/// The directory is created at `<workspace_root>/target/tmp/beet_tmp<uuid>`,
	/// where `<uuid>` is a randomly generated UUID v4. This is useful for keeping
	/// temporary files within the project structure, making them easier to locate
	/// and manage during development.
	///
	/// # Returns
	///
	/// Returns `Ok(TempDir)` if the directory was successfully created, or an
	/// error if directory creation failed or the workspace root could not be determined.
	///
	/// # Examples
	///
	/// ```no_run
	/// use beet_utils::path_utils::TempDir;
	///
	/// let temp = TempDir::new_workspace_relative()?;
	/// // Directory exists at <workspace>/target/tmp/beet_tmp<uuid>
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
	pub fn new_workspace_relative() -> FsResult<Self> {
		let workspace_root = fs_ext::workspace_root();
		let dir_name =
			format!("target/tmp/beet_tmp{}", Uuid::new_v4().to_string());
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

	/// Marks this temporary directory to be kept on drop.
	///
	/// By default, the directory is removed when the `TempDir` is dropped.
	/// Calling this method prevents the automatic cleanup, leaving the directory
	/// on the filesystem.
	///
	/// # Returns
	///
	/// Returns `self` for method chaining.
	///
	/// # Examples
	///
	/// ```no_run
	/// use beet_utils::path_utils::TempDir;
	///
	/// let temp = TempDir::new()?.keep();
	/// // Directory will NOT be deleted when temp goes out of scope
	/// # Ok::<(), Box<dyn std::error::Error>>(())
	/// ```
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

	#[test]
	fn test_tempdir_new_creates_and_cleans_up() {
		let dir_path;
		{
			// Create a temp directory
			let temp = TempDir::new().expect("Failed to create temp directory");
			dir_path = temp.path.clone();

			// Verify it exists
			assert!(
				dir_path.exists(),
				"Temp directory should exist after creation"
			);
			assert!(dir_path.is_dir(), "Temp path should be a directory");
		} // temp is dropped here

		// Verify it was cleaned up
		assert!(
			!dir_path.exists(),
			"Temp directory should be removed after drop"
		);
	}

	#[test]
	fn test_tempdir_workspace_relative_creates_and_cleans_up() {
		let dir_path;
		{
			// Create a workspace-relative temp directory
			let temp = TempDir::new_workspace_relative()
				.expect("Failed to create workspace-relative temp directory");
			dir_path = temp.path.clone();

			// Verify it exists
			assert!(
				dir_path.exists(),
				"Workspace-relative temp directory should exist after creation"
			);
			assert!(
				dir_path.is_dir(),
				"Workspace-relative temp path should be a directory"
			);

			// Verify it's in the workspace
			let path_str = dir_path.to_string();
			assert!(
				path_str.contains("target/tmp/beet_tmp"),
				"Directory should be in workspace target/tmp"
			);
		} // temp is dropped here

		// Verify it was cleaned up
		assert!(
			!dir_path.exists(),
			"Workspace-relative temp directory should be removed after drop"
		);
	}

	#[test]
	fn test_tempdir_unique_names() {
		// Create multiple temp directories and verify they have unique names
		let temp1 =
			TempDir::new().expect("Failed to create first temp directory");
		let temp2 =
			TempDir::new().expect("Failed to create second temp directory");

		assert_ne!(
			temp1.path, temp2.path,
			"Each temp directory should have a unique path"
		);
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
			assert!(
				dir_path.exists(),
				"Temp directory should exist after creation"
			);
		} // temp is dropped here

		// Verify it was NOT cleaned up because we called keep()
		assert!(
			dir_path.exists(),
			"Temp directory should still exist after drop when keep() is called"
		);

		// Manual cleanup for this test
		fs_ext::remove(&dir_path).ok();
	}
}
