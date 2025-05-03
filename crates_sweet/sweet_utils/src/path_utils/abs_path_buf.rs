use super::FsError;
use super::FsExt;
use super::FsResult;
use super::PathExt;
use crate::utils::PipelineTarget;
use path_clean::PathClean;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

/// Wrapper for `AbsPathBuf::new_workspace_rel(file!())`,
/// for use as a drop-in replacement for `file!()`.
/// ## Example
///
/// ```rust
/// # use sweet_utils::prelude::*;
/// let path = abs_file!();
/// ```
#[macro_export]
macro_rules! abs_file {
	() => {
		AbsPathBuf::new_workspace_rel(file!()).unwrap()
	};
}



/// A newtype `PathBuf` that makes several guarantees:
/// 1. the path is canonical
/// 2. on windows backslashes are replaced by forward slashes
/// 3. The hash is cross-platform as it uses encoded bytes
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsPathBuf(PathBuf);

impl Default for AbsPathBuf {
	fn default() -> Self {
		Self::new(std::env::current_dir().unwrap()).unwrap()
	}
}

impl AbsPathBuf {
	/// Create a new [`AbsPathBuf`] from a `PathBuf`.
	/// Canonicalization will prepend the `env::current_dir`,
	/// if your path is instead relative to the workspace root, ie `file!()`,
	/// use [`AbsPathBuf::new_workspace_rel`].
	///
	/// For wasm builds this just return the path as is.
	///
	/// ## Panics
	/// Panics if the path cannot be canonicalized. This will always be the case
	/// for wasm builds or if the path does not exist.
	///
	/// ## Example
	///
	/// ```rust
	/// # use sweet_utils::prelude::*;
	/// let path = AbsPathBuf::new("Cargo.toml");
	/// ```
	pub fn new(path: impl AsRef<Path>) -> FsResult<Self> {
		#[cfg(target_os = "windows")]
		{
			let canonical = PathExt::canonicalize(path)?;
			let canonical =
				canonical.to_string_lossy().replace('\\', "/").to_path_buf();
			Ok(Self(canonical))
		}
		#[cfg(not(target_os = "windows"))]
		{
			Ok(Self(PathExt::canonicalize(path)?))
		}
	}
	/// Create a new [`AbsPathBuf`] from a path relative to the workspace root,
	/// ie from using the `file!()` macro.
	/// ## Errors
	/// If the path cannot be canonicalized.
	pub fn new_workspace_rel(path: impl AsRef<Path>) -> FsResult<Self> {
		let path = FsExt::workspace_root().join(path);
		Self::new(path)
	}
	/// Create a new unchecked [`AbsPathBuf`] from a path relative to the workspace root,
	/// ie from using the `file!()` macro.
	pub fn new_workspace_rel_unchecked(path: impl AsRef<Path>) -> Self {
		let path = FsExt::workspace_root().join(path);
		Self::new_unchecked(path)
	}
	/// Create a new [`AbsPathBuf`] from a path relative to `CARGO_MANIFEST_DIR`,
	/// which will be the `crates/my_crate` dir in the case of a workspace.
	/// This is particularly useful inside of `build.rs` files.
	/// ## Errors
	/// If the path cannot be canonicalized.
	/// ## Panics
	/// Panics if `CARGO_MANIFEST_DIR` is not set.
	pub fn new_manifest_rel(path: impl AsRef<Path>) -> FsResult<Self> {
		std::env::var("CARGO_MANIFEST_DIR")
			.unwrap()
			.xref()
			.xmap(Path::new)
			.join(path)
			.xmap(Self::new)
	}
	/// Create a new unchecked [`AbsPathBuf`] from a path relative to `CARGO_MANIFEST_DIR`,
	/// which will be the `crates/my_crate` dir in the case of a workspace.
	/// This is particularly useful inside of `build.rs` files.
	/// ## Panics
	/// Panics if `CARGO_MANIFEST_DIR` is not set.
	pub fn new_manifest_rel_unchecked(path: impl AsRef<Path>) -> Self {
		std::env::var("CARGO_MANIFEST_DIR")
			.unwrap()
			.xref()
			.xmap(Path::new)
			.join(path)
			.xmap(Self::new_unchecked)
	}
	/// Create a new [`AbsPathBuf`] without canonicalizing, instead it is the users
	/// responsibility to ensure this path is already canonical.
	pub fn new_unchecked(path: impl AsRef<Path>) -> Self {
		let path = path.as_ref().clean();
		#[cfg(target_os = "windows")]
		{
			let canonical =
				path.to_string_lossy().replace('\\', "/").to_path_buf();
			Ok(Self(canonical))
		}
		Self(path)
	}
}
impl FromStr for AbsPathBuf {
	type Err = FsError;
	fn from_str(val: &str) -> Result<Self, Self::Err> { Self::new(val) }
}

impl AsRef<Path> for AbsPathBuf {
	fn as_ref(&self) -> &Path { self.0.as_ref() }
}

impl Into<PathBuf> for AbsPathBuf {
	fn into(self) -> PathBuf { self.0 }
}
impl Into<PathBuf> for &AbsPathBuf {
	fn into(self) -> PathBuf { self.0.to_path_buf() }
}

impl std::ops::Deref for AbsPathBuf {
	type Target = PathBuf;

	fn deref(&self) -> &Self::Target { &self.0 }
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	// use sweet_test::prelude::*;

	#[test]
	fn canonicalizes() { let _buf = AbsPathBuf::new("Cargo.toml").unwrap(); }

	#[test]
	fn abs_file() {
		assert!(abs_file!().to_string_lossy().ends_with("abs_path_buf.rs"));
	}
	#[test]
	fn workspace_rel() {
		let buf = AbsPathBuf::new_workspace_rel(file!()).unwrap();
		assert_eq!(buf, abs_file!());
	}
	#[test]
	fn manifest_rel() {
		let buf =
			AbsPathBuf::new_manifest_rel("src/path_utils/abs_path_buf.rs")
				.unwrap();
		assert_eq!(buf, abs_file!());
	}
}
