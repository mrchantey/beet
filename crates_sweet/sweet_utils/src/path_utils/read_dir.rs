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

	/// Get all dirs in a directory, not recursive
	pub fn dirs(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			dirs: true,
			..Default::default()
		}
		.read(root)
	}

	/// Get all files in a directory, not recursive
	pub fn files(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			files: true,
			..Default::default()
		}
		.read(root)
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

	/// Get all subdirectories recursively
	pub fn dirs_recursive(root: impl AsRef<Path>) -> FsResult<Vec<PathBuf>> {
		Self {
			dirs: true,
			recursive: true,
			..Default::default()
		}
		.read(root)
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
	) -> FsResult<()> {
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

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
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
		assert_eq!("../sweet_fs", a.to_str().unwrap());
		let _a =
			std::fs::read_dir(std::env::current_dir().unwrap().join("../"))
				.unwrap()
				.next()
				.unwrap()
				.unwrap()
				.path();
		// assert_eq!("../sweet_fs", a.to_str().unwrap());
	}


	#[test]
	fn fails() {
		let err_str = ReadDir::default()
			.read(FsExt::test_dir().join("foo"))
			.unwrap_err()
			.to_string()
			.replace("\\", "/");
		assert!(err_str.contains("test_dir/foo"));
	}

	#[test]
	fn dirs() {
		let err_str = ReadDir::dirs(FsExt::test_dir().join("foo"))
			.unwrap_err()
			.to_string()
			.replace("\\", "/");
		assert!(err_str.contains("test_dir/foo"));
		assert_eq!(ReadDir::dirs(FsExt::test_dir()).unwrap().len(), 2);
	}

	#[test]
	fn read_dir_recursive() {
		assert_eq!(
			ReadDir::dirs_recursive(FsExt::test_dir()).unwrap().len(),
			2
		);
	}

	#[test]
	fn files() {
		assert_eq!(ReadDir::files(FsExt::test_dir()).unwrap().len(), 3);
	}

	#[test]
	fn files_recursive() {
		assert_eq!(
			ReadDir::files_recursive(FsExt::test_dir()).unwrap().len(),
			5
		);
	}
}
