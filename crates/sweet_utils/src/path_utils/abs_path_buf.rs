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



/// A newtype `PathBuf` with several indications:
/// 1. the path is absolute, ie [`std::path::absolute`] is called
/// 2. the path is cleaned using [`path_clean`]
///
/// ## Serialization
/// Naturally serializing absolute paths is problematic for several reasons:
/// - moving the serialized path between machines will break
/// - often an `AbsPathBuf` is used for workspace config files, and workspace
/// 	paths are more intuitive in that context.  
/// For these reasons the path is serialized and deserialized relative to the workspace root,
/// using [`FsExt::workspace_root`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AbsPathBuf(PathBuf);

impl Default for AbsPathBuf {
	fn default() -> Self {
		Self::new(std::env::current_dir().unwrap()).unwrap()
	}
}

impl AbsPathBuf {
	/// Create a new [`AbsPathBuf`] from a `PathBuf`, calling [`std::path::absolute`]
	/// which prepends the `env::current_dir` to relative paths.
	/// If your path is instead relative to the workspace root, ie `file!()`,
	/// use [`AbsPathBuf::new_workspace_rel`].
	///
	/// For wasm builds this just return the path as is.
	///
	/// ## Errors
	/// Errors if calling [`std::path::absolute`] errors. This will always be the case
	/// for wasm builds or if the path does not exist.
	///
	/// ## Example
	///
	/// ```rust
	/// # use sweet_utils::prelude::*;
	/// let path = AbsPathBuf::new("Cargo.toml");
	/// ```
	pub fn new(path: impl AsRef<Path>) -> FsResult<Self> {
		let path = path.as_ref();
		let path = PathExt::absolute(path)?;
		let path = path.clean();
		Ok(Self(path))
	}

	/// Add a path to the current [`AbsPathBuf`], which will also naturally
	/// be an absolute path path.
	pub fn join(&self, path: impl AsRef<Path>) -> Self {
		let path = self.0.join(path).clean();
		Self(path)
	}


	pub fn with_extension(mut self, ext: &str) -> Self {
		self.0.set_extension(ext);
		self
	}

	/// Add a path to the current [`AbsPathBuf`], which will also naturally
	/// be a canonical path. This will error if the path cannot be canonicalized.
	pub fn join_checked(&self, path: impl AsRef<Path>) -> FsResult<Self> {
		let path = self.0.join(path);
		Self::new(path)
	}
	/// Create a new [`AbsPathBuf`] from a path relative to the workspace root,
	/// ie from using the `file!()` macro.
	/// ## Errors
	/// If the cwd cannot be resolved.
	/// ## Example
	///
	/// ```
	/// # use sweet_utils::prelude::*;
	/// let path = AbsPathBuf::new_workspace_rel(file!());
	/// ```
	pub fn new_workspace_rel(path: impl AsRef<Path>) -> FsResult<Self> {
		let path = FsExt::workspace_root().join(path);
		Self::new(path)
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

	/// Create a new [`AbsPathBuf`] verbatim from a path, its the user's
	/// responsibility to ensure that the path is absolute and cleaned.
	pub fn new_unchecked(path: impl AsRef<Path>) -> Self {
		let path = path.as_ref().clean();
		Self(path)
	}

	pub fn workspace_rel(
		&self,
	) -> FsResult<crate::path_utils::WorkspacePathBuf> {
		// Strip the workspace root from the path
		let path = PathExt::strip_prefix(&self.0, &FsExt::workspace_root())?;
		Ok(crate::path_utils::WorkspacePathBuf::new(path))
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
#[cfg(feature = "serde")]
impl serde::Serialize for AbsPathBuf {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		// Get workspace root
		let workspace_root = FsExt::workspace_root();

		// Make path relative to workspace root
		let rel_path = pathdiff::diff_paths(&self.0, &workspace_root)
			.ok_or_else(|| {
				serde::ser::Error::custom(
					"failed to make path relative to workspace root",
				)
			})?;

		// Serialize the relative path
		rel_path.serialize(serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for AbsPathBuf {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		// Deserialize to a PathBuf
		let rel_path = PathBuf::deserialize(deserializer)?;

		// Join with workspace root
		let abs_path = FsExt::workspace_root().join(rel_path);

		// Return as AbsPathBuf
		let abs_path = AbsPathBuf::new(abs_path).map_err(|err| {
			serde::de::Error::custom(format!(
				"failed to create AbsPathBuf: {}",
				err
			))
		})?;
		Ok(abs_path)
	}
}



#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;


	#[test]
	fn canonicalizes() { let _buf = AbsPathBuf::new("Cargo.toml").unwrap(); }

	#[test]
	fn resolves_relative_non_exist() {
		#[cfg(not(target_os = "windows"))]
		let expected = "foo/bar/boo.rs";
		#[cfg(target_os = "windows")]
		let expected = "foo\\bar\\boo.rs";

		assert!(
			AbsPathBuf::new("foo/bar/bazz/../boo.rs")
				.unwrap()
				.to_string_lossy()
				.ends_with(expected)
		);
	}
	#[test]
	fn abs_file() {
		assert!(abs_file!().to_string_lossy().ends_with("abs_path_buf.rs"));
	}
	#[test]
	fn workspace_rel() {
		let file = file!();
		let buf = AbsPathBuf::new_workspace_rel(file).unwrap();
		assert_eq!(buf, abs_file!());
		let workspace_rel = buf.workspace_rel().unwrap();
		assert_eq!(workspace_rel.to_string_lossy(), file);
	}
	#[test]
	fn manifest_rel() {
		let buf =
			AbsPathBuf::new_manifest_rel("src/path_utils/abs_path_buf.rs")
				.unwrap();
		assert_eq!(buf, abs_file!());
	}

	#[test]
	fn serde_roundtrip() {
		// Create an AbsPathBuf instance
		let original = abs_file!();

		// Serialize to JSON
		let serialized = serde_json::to_string(&original).unwrap();

		// Deserialize back to AbsPathBuf
		let deserialized: AbsPathBuf =
			serde_json::from_str(&serialized).unwrap();

		// Check if the roundtrip preserved the path
		assert_eq!(original, deserialized);
	}
}
