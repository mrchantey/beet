use super::FsError;
use super::FsExt;
use super::FsResult;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

pub struct PathExt;

impl PathExt {
	/// Create a path relative to the current working directory
	/// ## Errors
	/// If the current working directory cannot be determined
	pub fn relative(path: &impl AsRef<Path>) -> FsResult<&Path> {
		let cwd = FsExt::current_dir()?;
		PathExt::strip_prefix(path, &cwd)
	}

	/// Strip prefix
	pub fn strip_prefix<'a>(
		path: &'a impl AsRef<Path>,
		prefix: &impl AsRef<Path>,
	) -> FsResult<&'a Path> {
		path.as_ref()
			.strip_prefix(prefix)
			.map_err(|e| FsError::other(path.as_ref(), e))
	}

	/// Wraps [`Path::canonicalize`] error with a [`FsError`], actually
	/// outputting the path that caused the error.
	pub fn canonicalize(path: impl AsRef<Path>) -> FsResult<PathBuf> {
		path.as_ref()
			.canonicalize()
			.map_err(|e| FsError::io(path, e))
	}

	/// Create a relative path from a source to a destination:
	/// ## Example
	/// ```rust
	///	# use sweet_utils::prelude::*;
	/// # use std::path::PathBuf;
	/// assert_eq!(
	///		PathExt::create_relative("src", "src/lib.rs").unwrap(),
	///		PathBuf::from("lib.rs")
	/// );
	/// assert_eq!(
	///		PathExt::create_relative("foo/src", "foo/Cargo.toml").unwrap(),
	///		PathBuf::from("../Cargo.toml")
	///	);
	/// ```
	pub fn create_relative(
		src: impl AsRef<Path>,
		dst: impl AsRef<Path>,
	) -> FsResult<PathBuf> {
		let path = src.as_ref();
		let dst = dst.as_ref();
		pathdiff::diff_paths(dst, path).ok_or_else(|| {
			FsError::other(
				path,
				format!("Could not create relative path to dest: {:?}", dst),
			)
		})
	}

	pub fn to_forward_slash(path: impl AsRef<Path>) -> PathBuf {
		path.as_ref().to_string_lossy().replace("\\", "/").into()
	}

	pub fn file_stem(path: &impl AsRef<Path>) -> FsResult<&OsStr> {
		let path = path.as_ref();
		path.file_stem()
			.ok_or_else(|| FsError::other(path, "No file stem"))
	}
	pub fn file_name(path: &impl AsRef<Path>) -> FsResult<&OsStr> {
		let path = path.as_ref();
		path.file_name()
			.ok_or_else(|| FsError::other(path, "No file name"))
	}
	pub fn is_dir_or_extension(path: &impl AsRef<Path>, ext: &str) -> bool {
		let path = path.as_ref();
		match path.extension() {
			Some(value) => value.to_str().unwrap() == ext,
			None => path.is_dir(),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use std::path::PathBuf;

	#[test]
	fn works() {
		assert_eq!(
			PathExt::create_relative("src", "src/lib.rs").unwrap(),
			PathBuf::from("lib.rs")
		);
		assert_eq!(
			PathExt::create_relative("foo/bar/src", "foo/bar/Cargo.toml")
				.unwrap(),
			PathBuf::from("../Cargo.toml")
		);
	}
}
