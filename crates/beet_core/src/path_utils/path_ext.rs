//! Helpers for [`Path`] with ergonomics
//! better suited to the application layer:
//! - outputs the path on error
use crate::prelude::*;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

/// Create a path relative to the current working directory
/// ## Errors
/// If the current working directory cannot be determined
pub fn relative(path: &impl AsRef<Path>) -> FsResult<&Path> {
	let cwd = fs_ext::current_dir()?;
	path_ext::strip_prefix(path, &cwd)
}


pub fn join_relative(base: impl AsRef<Path>, rel: impl AsRef<Path>) -> PathBuf {
	let base = base.as_ref();
	let rel = rel.as_ref();
	let rel = rel.strip_prefix("/").unwrap_or(rel);
	base.join(rel)
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

/// Check if a path exists, returning an error if it does not.
pub fn assert_exists(path: impl AsRef<Path>) -> FsResult {
	let path = path.as_ref();
	if !path.exists() {
		return Err(FsError::FileNotFound {
			path: path.to_path_buf(),
		});
	}
	Ok(())
}

/// Wraps [`Path::canonicalize`] error with a [`FsError`],
/// outputting the path that caused the error.
pub fn canonicalize(path: impl AsRef<Path>) -> FsResult<PathBuf> {
	path.as_ref()
		.canonicalize()
		.map_err(|e| FsError::io(path, e))
}

/// Wraps [`std::path::absolute`] error with a [`FsError`],
/// outputting the path that caused the error.
///
/// On wasm platforms this will just ensure the path begins with a `/`
pub fn absolute(path: impl AsRef<Path>) -> FsResult<PathBuf> {
	let path = path.as_ref();
	#[cfg(not(target_arch = "wasm32"))]
	{
		std::path::absolute(path).map_err(|e| FsError::io(path, e))
	}
	#[cfg(target_arch = "wasm32")]
	{
		let path_str = path.to_string_lossy();
		if path_str.starts_with('/') {
			Ok(path.to_path_buf())
		} else {
			Ok(PathBuf::from(format!("/{}", path_str)))
		}
	}
}




/// Returns true if the path does not start with any of the absolute URL prefixes:
/// - `/`
/// - `http://`
/// - `https://`
/// - `file://`
/// - `data:`
/// - etc
pub fn is_relative_url(url: &str) -> bool {
	const ABS_PREFIXES: [&str; 15] = [
		"/",
		"http://",
		"https://",
		"file://",
		"data:",
		"mailto:",
		"tel:",
		"javascript:",
		"ftp://",
		"ws://",
		"wss://",
		"blob:",
		"cid:",
		"about:",
		"chrome:",
	];
	!ABS_PREFIXES.iter().any(|prefix| url.starts_with(prefix))
}

/// Create a relative path from a source to a destination:
/// ## Example
/// ```rust
///	# use beet_core::prelude::*;
/// # use std::path::PathBuf;
/// assert_eq!(
///		path_ext::create_relative("src", "src/lib.rs").unwrap(),
///		PathBuf::from("lib.rs")
/// );
/// assert_eq!(
///		path_ext::create_relative("foo/src", "foo/Cargo.toml").unwrap(),
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use std::path::PathBuf;

	#[test]
	fn works() {
		path_ext::create_relative("src", "src/lib.rs")
			.unwrap()
			.xpect_eq(PathBuf::from("lib.rs"));
		path_ext::create_relative("foo/bar/src", "foo/bar/Cargo.toml")
			.unwrap()
			.xpect_eq(PathBuf::from("../Cargo.toml"));
	}

	#[test]
	fn join_relative() {
		path_ext::join_relative("foo/bar", "baz/style.css")
			.xpect_eq(PathBuf::from("foo/bar/baz/style.css"));
		path_ext::join_relative("foo/bar", "/baz/style.css")
			.xpect_eq(PathBuf::from("foo/bar/baz/style.css"));
	}

	#[test]
	fn is_relative() {
		path_ext::is_relative_url("style.css").xpect_true();
		path_ext::is_relative_url("../style.css").xpect_true();
		path_ext::is_relative_url("/style.css").xpect_false();
		path_ext::is_relative_url("https://example.com").xpect_false();
	}
}
