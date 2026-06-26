//! `/`-separated path cleaning via [`clean`], plus (under `std`) ergonomic
//! filesystem [`Path`](std::path::Path) helpers that surface the offending
//! path on error.
use crate::prelude::*;

/// Lexically clean a `/`-separated path, mirroring the Plan 9 `cleanname`
/// procedure (the same algorithm the `path-clean` crate ports):
/// - collapse repeated slashes and drop `.` segments
/// - resolve `..` against the preceding named segment
/// - drop `..` at the root, but keep leading `..` on relative paths
/// - preserve a leading `/` on rooted paths
///
/// Returns `"."` for an empty result, matching current-directory semantics.
/// For the logically-relative [`SmolPath`] the leading `/` and `.` placeholder
/// are stripped by [`SmolPath::new`].
pub fn clean(input: &str) -> String {
	let rooted = input.starts_with('/');
	let mut out: Vec<&str> = Vec::new();
	for seg in input.split('/') {
		match seg {
			"" | "." => continue,
			".." => match out.last() {
				// collapse the preceding named segment
				Some(&last) if last != ".." => {
					out.pop();
				}
				// `..` at the root is a no-op
				_ if rooted => continue,
				// otherwise keep the leading `..`
				_ => out.push(".."),
			},
			other => out.push(other),
		}
	}
	let joined = out.join("/");
	if rooted {
		format!("/{joined}")
	} else if joined.is_empty() {
		".".to_string()
	} else {
		joined
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

#[cfg(feature = "std")]
pub use std_ext::*;

#[cfg(feature = "std")]
mod std_ext {
	use super::clean;
	use crate::prelude::*;
	use std::ffi::OsStr;
	use std::path::Path;
	use std::path::PathBuf;

	/// Lexically clean a filesystem [`Path`] via [`clean`]. On Windows the
	/// `\` separators are normalized to `/` so paths compare consistently
	/// across architectures.
	pub fn clean_path(path: impl AsRef<Path>) -> PathBuf {
		let raw = path.as_ref().to_string_lossy();
		#[cfg(target_os = "windows")]
		let raw = raw.replace('\\', "/");
		PathBuf::from(clean(&raw))
	}

	/// Create a path relative to the current working directory
	/// ## Errors
	/// If the current working directory cannot be determined
	pub fn relative(path: &impl AsRef<Path>) -> FsResult<&Path> {
		let cwd = fs_ext::current_dir()?;
		strip_prefix(path, &cwd)
	}

	/// Joins a base path with a relative path, stripping any leading `/` from the relative path.
	pub fn join_relative(
		base: impl AsRef<Path>,
		rel: impl AsRef<Path>,
	) -> PathBuf {
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
	/// On wasm a relative path is resolved against the runtime cwd
	/// ([`js_runtime::cwd`], ie `Deno.cwd()`), mirroring [`std::path::absolute`]'s
	/// cwd prefixing; an already-absolute path is returned as is. So an `FsStore`
	/// rooted at a workspace-relative entry resolves the same file native does.
	pub fn absolute(path: impl AsRef<Path>) -> FsResult<PathBuf> {
		let path = path.as_ref();
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				if path.is_absolute() {
					Ok(path.to_path_buf())
				} else {
					Ok(PathBuf::from(js_runtime::cwd()).join(path))
				}
			} else {
				std::path::absolute(path).map_err(|e| FsError::io(path, e))
			}
		}
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

	/// Converts backslashes to forward slashes in a path.
	pub fn to_forward_slash(path: impl AsRef<Path>) -> PathBuf {
		path.as_ref().to_string_lossy().replace("\\", "/").into()
	}

	/// Returns the file stem (name without extension), or an error if none.
	pub fn file_stem(path: &impl AsRef<Path>) -> FsResult<&OsStr> {
		let path = path.as_ref();
		path.file_stem()
			.ok_or_else(|| FsError::other(path, "No file stem"))
	}
	/// Returns the file name, or an error if none.
	pub fn file_name(path: &impl AsRef<Path>) -> FsResult<&OsStr> {
		let path = path.as_ref();
		path.file_name()
			.ok_or_else(|| FsError::other(path, "No file name"))
	}

	/// Returns `true` if the path is a directory or has the given extension.
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

	#[crate::test]
	fn cleans() {
		path_ext::clean("path//to///thing").xpect_eq("path/to/thing");
		path_ext::clean("/test/../path/").xpect_eq("/path");
		path_ext::clean("test/path/../../..").xpect_eq("..");
		path_ext::clean("/test/path/../../..").xpect_eq("/");
		path_ext::clean("../test").xpect_eq("../test");
		path_ext::clean("").xpect_eq(".");
		path_ext::clean("/").xpect_eq("/");
	}

	#[crate::test]
	#[cfg(feature = "std")]
	fn create_relative() {
		use std::path::PathBuf;
		path_ext::create_relative("src", "src/lib.rs")
			.unwrap()
			.xpect_eq(PathBuf::from("lib.rs"));
		path_ext::create_relative("foo/bar/src", "foo/bar/Cargo.toml")
			.unwrap()
			.xpect_eq(PathBuf::from("../Cargo.toml"));
	}

	#[crate::test]
	#[cfg(feature = "std")]
	fn join_relative() {
		use std::path::PathBuf;
		path_ext::join_relative("foo/bar", "baz/style.css")
			.xpect_eq(PathBuf::from("foo/bar/baz/style.css"));
		path_ext::join_relative("foo/bar", "/baz/style.css")
			.xpect_eq(PathBuf::from("foo/bar/baz/style.css"));
	}

	#[crate::test]
	fn is_relative() {
		path_ext::is_relative_url("style.css").xpect_true();
		path_ext::is_relative_url("../style.css").xpect_true();
		path_ext::is_relative_url("/style.css").xpect_false();
		path_ext::is_relative_url("https://example.com").xpect_false();
	}

	/// A relative path resolves to an absolute one (cwd-prefixed) and an
	/// already-absolute path round-trips. Cross-platform: on wasm the cwd comes
	/// from `js_runtime::cwd`, so a workspace-relative `FsStore` entry resolves the
	/// same file native does (regression: wasm used to only prepend a bare `/`).
	///
	/// Rootedness is asserted via a leading `/` rather than [`Path::is_absolute`]:
	/// `wasm32-unknown-unknown` has no platform path semantics so `is_absolute`
	/// always returns `false` there, the same reason [`clean`] keys off
	/// `starts_with('/')`.
	#[crate::test]
	#[cfg(feature = "std")]
	fn absolute() {
		let resolved = path_ext::absolute("foo/bar.txt").unwrap();
		resolved.to_string_lossy().starts_with('/').xpect_true();
		resolved.ends_with("foo/bar.txt").xpect_true();
		path_ext::absolute("/already/abs.txt")
			.unwrap()
			.xpect_eq(std::path::PathBuf::from("/already/abs.txt"));
	}
}
