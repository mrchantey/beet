#[allow(unused_imports)]
use crate::prelude::*;
use path_clean::PathClean;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;




/// ## Workspace PathBuf
/// A newtype with several indications:
/// 1. the path is relative to the workspace root
/// 2. the path is cleaned using [`path_clean`]
/// 3. on windows backslashes are replaced by forward slashes
///    - This is done to ensure exact matches because this type is often used across architectures.
///
/// The path does **not** have to exist
///
/// ## Example
///
/// ```rust
/// # use beet_utils::prelude::*;
/// let path = WsPathBuf::new(file!());
///
/// ```
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Hash, bevy::reflect::Reflect,
)]
pub struct WsPathBuf(
	// TODO upstream Pathbuf Reflect
	PathBuf,
);
impl std::fmt::Display for WsPathBuf {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0.to_string_lossy())
	}
}

impl WsPathBuf {
	/// Create a new [`WsPathBuf`], a common use case is to use `file!()`
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
	/// Panics if [`fs_ext::workspace_root`] fails.
	pub fn new_cwd_rel(path: impl AsRef<Path>) -> FsResult<Self> {
		let path = path_ext::absolute(path)?;
		// TODO use pathdiff instead?
		let path = path_ext::strip_prefix(&path, &fs_ext::workspace_root())?;
		Ok(Self::new(path))
	}

	pub fn take(self) -> PathBuf { self.0 }

	/// Create a new [`WsPathBuf`] from joining this one with
	/// another [`Path`]
	pub fn join(&self, path: impl AsRef<Path>) -> Self {
		let path = self.0.join(path).clean();
		Self::new(path)
	}

	/// Convert to a [`AbsPathBuf`]. This should be used instead of
	/// canonicalize/path::absolute because they prepend cwd instead of
	/// workspace root.
	///
	/// # Panics
	/// Panics if the workspace root or cwd cannot be determined.
	pub fn into_abs(&self) -> AbsPathBuf {
		let path = fs_ext::workspace_root().join(self).clean();
		AbsPathBuf::new(path)
			.map_err(|err| {
				format!("Failed to convert WsPathBuf to AbsPathBuf: {err}")
			})
			.unwrap()
	}
}

impl std::ops::Deref for WsPathBuf {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for WsPathBuf {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl AsRef<Path> for WsPathBuf {
	fn as_ref(&self) -> &Path { self.0.as_ref() }
}
impl FromStr for WsPathBuf {
	type Err = FsError;
	fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(Self::new(s)) }
}
impl Into<WsPathBuf> for &str {
	fn into(self) -> WsPathBuf { WsPathBuf::new(self) }
}
impl Into<WsPathBuf> for PathBuf {
	fn into(self) -> WsPathBuf { WsPathBuf::new(self) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use std::path::PathBuf;

	#[test]
	fn works() {
		assert_eq!(
			WsPathBuf::new("Cargo.toml").as_path(),
			PathBuf::from("Cargo.toml").as_path()
		);
		assert_eq!(
			WsPathBuf::new("foo/../Cargo.toml").as_path(),
			PathBuf::from("Cargo.toml").as_path()
		);
	}
}
