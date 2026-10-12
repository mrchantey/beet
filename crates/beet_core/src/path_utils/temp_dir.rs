use crate::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// A uniquely named directory, removed when dropped.
///
/// The name carries a UUID v7, so concurrent processes (two runs of one test
/// binary) never share a directory. [`TempDir::new_ws`] works on every target
/// `fs_ext` does, [`TempDir::new`] (the system temp dir) natively only.
///
/// Not [`Clone`], as it removes the directory on drop.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// let temp = TempDir::new_ws().unwrap();
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

#[cfg(not(target_arch = "wasm32"))]
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
	#[cfg(not(target_arch = "wasm32"))]
	pub fn new() -> FsResult<Self> {
		let temp_dir = std::env::temp_dir();
		let dir_name = format!("beet_tmp_{}", uuid_ext::now_v7());
		let dir_path = temp_dir.join(dir_name);
		Self::new_with_path(dir_path)
	}

	/// Creates a new temporary directory relative to the workspace root, at
	/// `<workspace_root>/target/tmp/beet_tmp_<uuid>`, for temporary files
	/// that should stay within the project structure. The `CARGO_TARGET_DIR`
	/// env var is not used.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	///
	/// let temp = TempDir::new_ws().unwrap();
	/// ```
	pub fn new_ws() -> FsResult<Self> {
		let workspace_root = fs_ext::workspace_root();
		let dir_name = format!("target/tmp/beet_tmp_{}", uuid_ext::now_v7());
		let dir_path = workspace_root.join(dir_name);
		Self::new_with_path(dir_path)
	}

	fn new_with_path(path: PathBuf) -> FsResult<Self> {
		if fs_ext::exists(&path)? {
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

	/// Creates `temp`, asserts it exists, drops it, and returns its path.
	fn create_and_drop(temp: TempDir) -> AbsPath {
		let path = temp.path().clone();
		fs_ext::is_dir(&path).xpect_true();
		drop(temp);
		path
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn system_dir_cleans_up() {
		create_and_drop(TempDir::new().unwrap())
			.xmap(fs_ext::exists)
			.unwrap()
			.xpect_false();
	}

	#[crate::test]
	fn workspace_dir_cleans_up() {
		let path = create_and_drop(TempDir::new_ws().unwrap());
		path.contains("target/tmp/beet_tmp_").xpect_true();
		fs_ext::exists(&path).unwrap().xpect_false();
	}

	#[crate::test]
	fn unique_names() {
		let first = TempDir::new_ws().unwrap();
		let second = TempDir::new_ws().unwrap();
		first.path().xpect_not_eq(second.path().clone());
	}

	#[crate::test]
	fn keep_prevents_cleanup() {
		let path = create_and_drop(TempDir::new_ws().unwrap().keep());
		fs_ext::exists(&path).unwrap().xpect_true();
		fs_ext::remove(&path).unwrap();
	}
}
