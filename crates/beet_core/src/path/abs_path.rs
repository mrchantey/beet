use crate::prelude::*;
use core::ops::Deref;

/// Wrapper for `AbsPath::new_workspace_rel(file!())`,
/// for use as a drop-in replacement for `file!()`.
/// ## Example
///
/// ```rust
/// # use beet_core::prelude::*;
/// let path = abs_file!();
/// ```
#[cfg(feature = "std")]
#[macro_export]
macro_rules! abs_file {
	() => {
		AbsPath::new_workspace_rel(file!()).unwrap()
	};
}

/// An absolute filesystem location: a rooted, cleaned [`SmolPath`].
///
/// The type is `no_std` so a path can be authored and carried anywhere (a
/// [`StoreUri`] pinned to a directory, a [`ChildProcess`] cwd); only resolving
/// one against the current directory or the workspace root
/// ([`new`](Self::new), [`new_workspace_rel`](Self::new_workspace_rel)) needs
/// `std`.
///
/// ## Serialization
/// Naturally serializing absolute paths is problematic, moving the serialized
/// path between machines will break. Instead the path is serialized and
/// deserialized relative to the workspace root, using
/// [`fs_ext::workspace_root`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(
	feature = "tokens",
	derive(ToTokens),
	to_tokens(AbsPath::new_unchecked)
)]
pub struct AbsPath(SmolPath);

impl AbsPath {
	/// Create a new [`AbsPath`] verbatim from a path, cleaning it: it is the
	/// caller's responsibility to ensure the path is absolute.
	pub fn new_unchecked(path: impl AsRef<str>) -> Self {
		Self(SmolPath::new(path))
	}

	/// Append a path below this one, resolving any `..` segments. The join is
	/// always below: a leading `/` on `path` is discarded rather than
	/// replacing this path.
	pub fn join(&self, path: impl AsRef<str>) -> Self {
		Self(self.0.join(path))
	}

	/// Returns the parent directory, or [`None`] at the root.
	pub fn parent(&self) -> Option<Self> { self.0.parent().map(Self) }

	/// Replaces the file extension with the given value.
	pub fn with_extension(self, ext: &str) -> Self {
		Self(self.0.with_extension(ext))
	}

	/// This path relative to `base`, [`None`] when `base` is neither this path
	/// nor an ancestor of it.
	pub fn strip_prefix(&self, base: &Self) -> Option<RelPath> {
		self.0.strip_prefix(&base.0).map(RelPath::new)
	}

	/// Borrow the inner [`SmolPath`].
	pub fn as_smol_path(&self) -> &SmolPath { &self.0 }

	/// Consume into the inner [`SmolPath`].
	pub fn into_smol_path(self) -> SmolPath { self.0 }
}

impl Deref for AbsPath {
	type Target = SmolPath;
	fn deref(&self) -> &SmolPath { &self.0 }
}

impl core::fmt::Display for AbsPath {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl AsRef<str> for AbsPath {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

impl AsRef<SmolPath> for AbsPath {
	fn as_ref(&self) -> &SmolPath { &self.0 }
}

impl From<AbsPath> for SmolPath {
	fn from(value: AbsPath) -> Self { value.0 }
}

impl From<&AbsPath> for SmolPath {
	fn from(value: &AbsPath) -> Self { value.0.clone() }
}

// resolving against the current directory or workspace root is a std surface
#[cfg(feature = "std")]
mod std_ext {
	use super::AbsPath;
	use crate::prelude::*;
	use core::str::FromStr;
	use std::path::Path;
	use std::path::PathBuf;

	impl AbsPath {
		/// Create a new [`AbsPath`], calling [`path_ext::absolute`] which
		/// prepends the current directory to a relative path. If your path is
		/// instead relative to the workspace root, ie `file!()`, use
		/// [`AbsPath::new_workspace_rel`].
		///
		/// ## Errors
		/// Errors if the current directory cannot be determined.
		///
		/// ## Example
		///
		/// ```rust
		/// # use beet_core::prelude::*;
		/// let path = AbsPath::new("Cargo.toml");
		/// ```
		pub fn new(path: impl AsRef<Path>) -> FsResult<Self> {
			path_ext::absolute(path)?
				.to_string_lossy()
				.xmap(Self::new_unchecked)
				.xok()
		}

		/// Create a new [`AbsPath`] from a path relative to the workspace root,
		/// ie from using the `file!()` macro.
		/// ## Errors
		/// If the cwd cannot be resolved.
		/// ## Example
		///
		/// ```
		/// # use beet_core::prelude::*;
		/// let path = AbsPath::new_workspace_rel(file!());
		/// ```
		pub fn new_workspace_rel(path: impl AsRef<Path>) -> FsResult<Self> {
			Self::new(fs_ext::workspace_root())
				.map(|abs| abs.join(path.as_ref().to_string_lossy()))
		}

		/// Create a new [`AbsPath`] from a path relative to `CARGO_MANIFEST_DIR`,
		/// which will be the `crates/my_crate` dir in the case of a workspace.
		/// This is particularly useful inside of `build.rs` files.
		/// ## Errors
		/// If the cwd cannot be resolved.
		/// ## Panics
		/// Panics if `CARGO_MANIFEST_DIR` is not set.
		pub fn new_manifest_rel(path: impl AsRef<Path>) -> FsResult<Self> {
			env_ext::var("CARGO_MANIFEST_DIR")
				.unwrap()
				.as_str()
				.xmap(Path::new)
				.join(path)
				.xmap(Self::new)
		}

		/// Converts this absolute path to a workspace-relative path.
		pub fn into_ws_path(&self) -> FsResult<WsPath> {
			self.strip_prefix(&Self::workspace_root())
				.map(WsPath::new)
				.ok_or_else(|| FsError::ExpectedWorkspaceRelative {
					path: self.to_path_buf(),
				})
		}

		/// The workspace root as an [`AbsPath`], see [`fs_ext::workspace_root`].
		/// Resolved against the cwd, so a host with no workspace (a browser,
		/// where the root is empty) still yields a rooted path.
		///
		/// ## Panics
		/// Panics if the cwd cannot be determined.
		pub fn workspace_root() -> Self {
			Self::new(fs_ext::workspace_root()).unwrap()
		}
	}

	/// The current directory.
	impl Default for AbsPath {
		fn default() -> Self {
			Self::new(fs_ext::current_dir().unwrap()).unwrap()
		}
	}

	impl FromStr for AbsPath {
		type Err = FsError;
		fn from_str(val: &str) -> Result<Self, Self::Err> { Self::new(val) }
	}

	impl AsRef<Path> for AbsPath {
		fn as_ref(&self) -> &Path { Path::new(self.0.as_str()) }
	}

	impl From<AbsPath> for PathBuf {
		fn from(value: AbsPath) -> Self { PathBuf::from(value.0.as_str()) }
	}

	impl From<&AbsPath> for PathBuf {
		fn from(value: &AbsPath) -> Self { PathBuf::from(value.0.as_str()) }
	}

	impl From<WsPath> for AbsPath {
		fn from(value: WsPath) -> Self { value.into_abs() }
	}

	impl From<&WsPath> for AbsPath {
		fn from(value: &WsPath) -> Self { value.into_abs() }
	}

	#[cfg(feature = "serde")]
	impl serde::Serialize for AbsPath {
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
		{
			pathdiff::diff_paths(self, fs_ext::workspace_root())
				.ok_or_else(|| {
					serde::ser::Error::custom(
						"failed to make path relative to workspace root",
					)
				})?
				.to_string_lossy()
				.xmap(SmolPath::new)
				.serialize(serializer)
		}
	}

	#[cfg(feature = "serde")]
	impl<'de> serde::Deserialize<'de> for AbsPath {
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: serde::Deserializer<'de>,
		{
			let rel_path = SmolPath::deserialize(deserializer)?;
			AbsPath::new(fs_ext::workspace_root().join(rel_path)).map_err(
				|err| {
					serde::de::Error::custom(format!(
						"failed to create AbsPath: {}",
						err
					))
				},
			)
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn join_and_parent() {
		let path = AbsPath::new_unchecked("/srv/site");
		path.join("/assets").as_str().xpect_eq("/srv/site/assets");
		path.join("../other").as_str().xpect_eq("/srv/other");
		path.parent().unwrap().as_str().xpect_eq("/srv");
		AbsPath::new_unchecked("/").parent().xpect_none();
	}

	#[crate::test]
	fn strip_prefix() {
		let base = AbsPath::new_unchecked("/srv/site");
		base.join("a/b.txt")
			.strip_prefix(&base)
			.xpect_eq(Some(RelPath::new("a/b.txt")));
		AbsPath::new_unchecked("/other")
			.strip_prefix(&base)
			.xpect_none();
	}
}

#[cfg(test)]
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
mod std_test {
	use crate::prelude::*;

	#[crate::test]
	fn resolves_relative() {
		AbsPath::new("Cargo.toml")
			.unwrap()
			.is_absolute()
			.xpect_true();
		AbsPath::new("foo/bar/bazz/../boo.rs")
			.unwrap()
			.ends_with("foo/bar/boo.rs")
			.xpect_true();
	}

	#[crate::test]
	fn abs_file() { abs_file!().ends_with("abs_path.rs").xpect_true(); }

	#[crate::test]
	fn workspace_rel() {
		let file = file!();
		let buf = AbsPath::new_workspace_rel(file).unwrap();
		buf.xpect_eq(abs_file!());
		buf.into_ws_path().unwrap().as_str().xpect_eq(file);
		// a leading slash is still workspace-relative
		AbsPath::new_workspace_rel(format!("/{file}"))
			.unwrap()
			.xpect_eq(abs_file!());
	}

	#[crate::test]
	fn manifest_rel() {
		AbsPath::new_manifest_rel("src/path/abs_path.rs")
			.unwrap()
			.xpect_eq(abs_file!());
	}

	#[crate::test]
	#[cfg(feature = "json")]
	fn serde_roundtrip() {
		let original = abs_file!();
		let serialized = serde_json::to_string(&original).unwrap();
		serialized.xpect_eq(format!("\"{}\"", file!()));
		let deserialized: AbsPath = serde_json::from_str(&serialized).unwrap();
		original.xpect_eq(deserialized);
	}
}
