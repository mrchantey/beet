#[allow(unused_imports)]
use crate::prelude::*;
use path_clean::PathClean;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;





/// A newtype `PathBuf` with several indications:
/// 1. the path is relative to the workspace root
/// 2. the path is cleaned using [`path_clean`]
/// 3. on windows backslashes are replaced by forward slashes
///
/// The path does **not** have to exist
///
/// ## Example
///
/// ```rust
/// # use sweet_utils::prelude::*;
/// let path = WorkspacePathBuf::new(file!());
///
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bevy", derive(bevy::reflect::Reflect))]
pub struct WorkspacePathBuf(
	// TODO upstream Pathbuf Reflect
	PathBuf,
);


impl WorkspacePathBuf {
	/// Create a new [`WorkspacePathBuf`], a common use case is to use `file!()`
	/// which is already relative to the workspace root.
	pub fn new(path: impl AsRef<Path>) -> Self {
		let path = path.as_ref();
		#[cfg(target_os = "windows")]
		let path = PathBuf::from(path.to_string_lossy().replace('\\', "/"));
		let path = path.clean();
		Self(path)
	}

	/// Using calls like `std::fs::read_dir` will return paths relative
	/// to the current directory of the process, not the workspace root.
	/// This function will resolve the difference by first canonicalizing
	/// the path and then stripping the workspace root.
	/// ## Panics
	///
	/// Panics if [`FsExt::workspace_root`] fails.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn new_from_cwd_rel(path: impl AsRef<Path>) -> FsResult<Self> {
		use crate::prelude::PathExt;
		let path = PathExt::absolute(path)?;
		let workspace_root = FsExt::workspace_root();
		// TODO use pathdiff instead?
		let path = path.strip_prefix(workspace_root).map_err(|err| {
			FsError::InvalidPath {
				path: path.to_path_buf(),
				err: err.to_string(),
			}
		})?;
		Ok(Self::new(path))
	}

	/// Create a new [`WorkspacePathBuf`] from joining this one with
	/// another [`Path`]
	pub fn join(&self, path: impl AsRef<Path>) -> Self {
		let path = self.0.join(path);
		Self::new(path)
	}

	#[cfg(not(target_arch = "wasm32"))]
	/// Convert to a [`AbsPathBuf`]. This should be used instead of
	/// canonicalize/path::absolute because they prepend cwd instead of
	/// workspace root.
	///
	/// # Panics
	/// Panics if the workspace root or cwd cannot be determined.
	pub fn into_abs(&self) -> AbsPathBuf {
		let path = FsExt::workspace_root().join(self).clean();
		AbsPathBuf::new(path)
			.map_err(|err| {
				anyhow::anyhow!(
					"Failed to convert WorkspacePathBuf to AbsPathBuf: {}",
					err
				)
			})
			.unwrap()
	}

}

impl std::ops::Deref for WorkspacePathBuf {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target { &self.0 }
}
impl AsRef<Path> for WorkspacePathBuf {
	fn as_ref(&self) -> &Path { self.0.as_ref() }
}
impl FromStr for WorkspacePathBuf {
	type Err = anyhow::Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(Self::new(s)) }
}
impl Into<WorkspacePathBuf> for &str {
	fn into(self) -> WorkspacePathBuf { WorkspacePathBuf::new(self) }
}
impl Into<WorkspacePathBuf> for PathBuf {
	fn into(self) -> WorkspacePathBuf { WorkspacePathBuf::new(self) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use std::path::PathBuf;

	#[test]
	fn works() {
		assert_eq!(
			WorkspacePathBuf::new("Cargo.toml").as_path(),
			PathBuf::from("Cargo.toml").as_path()
		);
		assert_eq!(
			WorkspacePathBuf::new("foo/../Cargo.toml").as_path(),
			PathBuf::from("Cargo.toml").as_path()
		);
	}
}
