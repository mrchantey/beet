use crate::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Read a directory or file into a Vec<PathBuf>.
/// All options are false by default.
/// All paths will include the root.
#[derive(Debug)]
pub struct ReadDir {
	/// include files
	pub files: bool,
	/// include directories
	pub dirs: bool,
	/// search subdirectories
	pub recursive: bool,
	/// include the root directory
	pub root: bool,
}

impl Default for ReadDir {
	fn default() -> Self {
		Self {
			files: false,
			dirs: false,
			recursive: false,
			root: false,
		}
	}
}

impl ReadDir {
	/// Get all files and directories in a directory, not recursive
	pub fn all(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			files: true,
			dirs: true,
			..Default::default()
		}
		.read(root)
	}

	/// Async: Get all files and directories in a directory, not recursive
	pub async fn all_async(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
		{
			Self::all(root)
		}
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			Self {
				files: true,
				dirs: true,
				..Default::default()
			}
			.read_async(root)
			.await
		}
	}

	/// Get all dirs in a directory, not recursive
	pub fn dirs(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			dirs: true,
			..Default::default()
		}
		.read(root)
	}

	/// Async: Get all dirs in a directory, not recursive
	pub async fn dirs_async(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
		{
			Self::dirs(root)
		}
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			Self {
				dirs: true,
				..Default::default()
			}
			.read_async(root)
			.await
		}
	}

	/// Get all files in a directory, not recursive
	pub fn files(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			files: true,
			..Default::default()
		}
		.read(root)
	}

	/// Async: Get all files in a directory, not recursive
	pub async fn files_async(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
		{
			Self::files(root)
		}
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			Self {
				files: true,
				..Default::default()
			}
			.read_async(root)
			.await
		}
	}

	/// Get all files and directories recursively
	pub fn all_recursive(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			dirs: true,
			files: true,
			recursive: true,
			..Default::default()
		}
		.read(root)
	}

	/// Async: Get all files and directories recursively
	pub async fn all_recursive_async(
		root: impl AsRef<Path>,
	) -> FsResult<Vec<PathBuf>> {
		#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
		{
			Self::all_recursive(root)
		}
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			Self {
				dirs: true,
				files: true,
				recursive: true,
				..Default::default()
			}
			.read_async(root)
			.await
		}
	}

	/// Get all subdirectories recursively
	pub fn dirs_recursive(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			dirs: true,
			recursive: true,
			..Default::default()
		}
		.read(root)
	}

	/// Async: Get all subdirectories recursively
	pub async fn dirs_recursive_async(
		root: impl AsRef<Path>,
	) -> FsResult<Vec<PathBuf>> {
		#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
		{
			Self::dirs_recursive(root)
		}
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			Self {
				dirs: true,
				recursive: true,
				..Default::default()
			}
			.read_async(root)
			.await
		}
	}

	/// Get all files recursively
	pub fn files_recursive(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			files: true,
			recursive: true,
			..Default::default()
		}
		.read(root)
	}

	/// Async: Get all files recursively
	pub async fn files_recursive_async(
		root: impl AsRef<Path>,
	) -> FsResult<Vec<PathBuf>> {
		#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
		{
			Self::files_recursive(root)
		}
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			Self {
				files: true,
				recursive: true,
				..Default::default()
			}
			.read_async(root)
			.await
		}
	}


	/// Read dir with the provided options. if the root is a file, the
	/// file will be returned.
	pub fn read(&self, root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		let mut paths = Vec::new();
		if self.root {
			paths.push(root.as_ref().to_path_buf());
		}
		self.read_inner(root, &mut paths)?;
		Ok(paths)
	}
	fn read_inner(
		&self,
		file_or_dir: impl AsRef<Path>,
		paths: &mut Vec<PathBuf>,
	) -> FsResult {
		let path = file_or_dir.as_ref();
		if path.is_file() {
			if self.files {
				paths.push(path.to_path_buf());
			}
			return Ok(());
		}
		let children = fs::read_dir(path).map_err(|e| FsError::io(path, e))?;
		for child in children {
			let child = child
				.map_err(|err| FsError::ChildIo {
					parent: path.into(),
					err,
				})
				.map(|c| c.path())?;
			if child.is_dir() {
				if self.dirs {
					paths.push(child.clone());
				}
				if self.recursive {
					self.read_inner(child, paths)?;
				}
			} else if child.is_file() && self.files {
				paths.push(child.clone());
			} else {
				// ignore
			}
		}
		Ok(())
	}

	/// Async version: Read dir with the provided options. if the root is a file, the
	/// file will be returned.
	pub async fn read_async(
		&self,
		root: impl AsRef<Path>,
	) -> FsResult<Vec<PathBuf>> {
		#[cfg(not(all(feature = "fs", not(target_arch = "wasm32"))))]
		{
			self.read(root)
		}
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		{
			let mut paths = Vec::new();
			if self.root {
				paths.push(root.as_ref().to_path_buf());
			}
			self.read_inner_async(root, &mut paths).await?;
			Ok(paths)
		}
	}

	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	async fn read_inner_async(
		&self,
		file_or_dir: impl AsRef<Path>,
		paths: &mut Vec<PathBuf>,
	) -> FsResult {
		let root = file_or_dir.as_ref().to_path_buf();
		let mut stack = vec![root];

		while let Some(path) = stack.pop() {
			use futures_lite::StreamExt;

			let metadata = async_fs::metadata(&path)
				.await
				.map_err(|e| FsError::io(&path, e))?;

			if metadata.is_file() {
				if self.files {
					paths.push(path.clone());
				}
				continue;
			}

			let mut read_dir = match async_fs::read_dir(&path).await {
				Ok(rd) => rd,
				Err(e) => return Err(FsError::io(&path, e)),
			};

			while let Some(entry) = read_dir.next().await {
				let child = entry
					.map_err(|err| FsError::ChildIo {
						parent: path.clone().into(),
						err,
					})?
					.path();
				let child_metadata = async_fs::metadata(&child)
					.await
					.map_err(|e| FsError::io(&child, e))?;

				if child_metadata.is_dir() {
					if self.dirs {
						paths.push(child.clone());
					}
					if self.recursive {
						stack.push(child);
					}
				} else if child_metadata.is_file() && self.files {
					paths.push(child.clone());
				} else {
					// ignore
				}
			}
		}
		Ok(())
	}

	/// Read dir recursive for each path, ignoring DirNotFound errors
	pub fn read_dirs_ok(
		&self,
		paths: impl IntoIterator<Item = impl AsRef<Path>>,
	) -> FsResult<Vec<PathBuf>> {
		let mut vec = Vec::new();
		for path in paths {
			match self.read(path.as_ref()) {
				Ok(val) => {
					vec.extend(val);
				}
				// do nothing
				Err(FsError::DirNotFound { .. }) => {}
				Err(err) => return Err(err),
			};
		}
		Ok(vec)
	}
}

#[cfg(all(test, feature = "fs", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;

	#[test]
	#[ignore = "just experiments"]
	fn relative_to() {
		let a = std::fs::read_dir("../")
			.unwrap()
			.next()
			.unwrap()
			.unwrap()
			.path();
		a.to_str().unwrap().xpect_eq("../beet_core");
		let _a =
			std::fs::read_dir(std::env::current_dir().unwrap().join("../"))
				.unwrap()
				.next()
				.unwrap()
				.unwrap()
				.path();
		// a.to_str().unwrap().xpect_eq("../beet_core");
	}

	#[test]
	fn fails() {
		let err_str = ReadDir::default()
			.read(fs_ext::test_dir().join("foo"))
			.unwrap_err()
			.to_string()
			.replace("\\", "/");
		err_str.contains("test_dir/foo").xpect_true();
	}

	#[test]
	fn dirs() {
		let err_str = ReadDir::dirs(fs_ext::test_dir().join("foo"))
			.unwrap_err()
			.to_string()
			.replace("\\", "/");
		err_str.contains("test_dir/foo").xpect_true();
		ReadDir::dirs(fs_ext::test_dir()).unwrap().len().xpect_eq(2);
	}

	#[test]
	fn read_dir_recursive() {
		ReadDir::dirs_recursive(fs_ext::test_dir())
			.unwrap()
			.len()
			.xpect_eq(2);
	}

	#[test]
	fn files() {
		ReadDir::files(fs_ext::test_dir())
			.unwrap()
			.len()
			.xpect_eq(3);
	}

	#[test]
	fn files_recursive() {
		ReadDir::files_recursive(fs_ext::test_dir())
			.unwrap()
			.len()
			.xpect_eq(5);
	}
}

#[cfg(all(test, feature = "fs", not(target_arch = "wasm32")))]
mod test_async {
	use crate::prelude::*;

	#[sweet::test]
	async fn fails() {
		let err_str = ReadDir::default()
			.read_async(fs_ext::test_dir().join("foo"))
			.await
			.unwrap_err()
			.to_string()
			.replace("\\", "/");
		err_str.contains("test_dir/foo").xpect_true();
	}

	#[sweet::test]
	async fn dirs() {
		let err_str = ReadDir::dirs_async(fs_ext::test_dir().join("foo"))
			.await
			.unwrap_err()
			.to_string()
			.replace("\\", "/");
		err_str.contains("test_dir/foo").xpect_true();
		ReadDir::dirs_async(fs_ext::test_dir())
			.await
			.unwrap()
			.len()
			.xpect_eq(2);
	}

	#[sweet::test]
	async fn read_dir_recursive() {
		ReadDir::dirs_recursive_async(fs_ext::test_dir())
			.await
			.unwrap()
			.len()
			.xpect_eq(2);
	}

	#[sweet::test]
	async fn files() {
		ReadDir::files_async(fs_ext::test_dir())
			.await
			.unwrap()
			.len()
			.xpect_eq(3);
	}

	#[sweet::test]
	async fn files_recursive() {
		ReadDir::files_recursive_async(fs_ext::test_dir())
			.await
			.unwrap()
			.len()
			.xpect_eq(5);
	}
}
