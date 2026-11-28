use crate::prelude::*;
use core::ops::Deref;
use core::str::FromStr;

/// A location relative to the workspace root, usually the directory
/// containing the main `Cargo.toml`: a [`FwdPath`] below that root, so it
/// never names anything outside the workspace. When used as a component this
/// indicates the entity represents a file or directory with the given path,
/// which does **not** have to exist.
///
/// The type is `no_std`; resolving one to an [`AbsPath`]
/// ([`into_abs`](Self::into_abs)) needs the workspace root and so `std`.
///
/// ## Example
///
/// ```rust
/// # use beet_core::prelude::*;
/// let path = WsPath::new("crates/beet_core/Cargo.toml").unwrap();
/// WsPath::new("/etc/hosts").unwrap_err();
/// ```
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	Component,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
	feature = "serde",
	serde(try_from = "SmolStr", into = "SmolStr")
)]
#[cfg_attr(feature = "tokens", derive(ToTokens), to_tokens(WsPath::from_fwd_path))]
pub struct WsPath(FwdPath);

impl WsPath {
	/// Parse a path relative to the workspace root, ie `file!()`.
	///
	/// # Errors
	/// Errors as [`FwdPath::new`]: on a leading `/`, or a `..` climbing out
	/// of the workspace.
	pub fn new(path: impl AsRef<str>) -> Result<Self> {
		FwdPath::new(path).map(Self)
	}

	/// The workspace location `path` names below the root.
	pub const fn from_fwd_path(path: FwdPath) -> Self { Self(path) }

	/// The path `path` below this one, which can never climb out of it.
	pub fn join(&self, path: &FwdPath) -> Self { Self(self.0.join(path)) }

	/// [`join`](Self::join) a path parsed from text.
	///
	/// # Errors
	/// Errors when `path` is rooted or climbs above this one.
	pub fn try_join(&self, path: impl AsRef<str>) -> Result<Self> {
		self.0.try_join(path).map(Self)
	}

	/// Returns the parent directory, or [`None`] at the workspace root.
	pub fn parent(&self) -> Option<Self> { self.0.parent().map(Self) }

	/// Borrow the path below the workspace root.
	pub fn as_fwd_path(&self) -> &FwdPath { &self.0 }

	/// Borrow the inner [`SmolPath`].
	pub fn as_smol_path(&self) -> &SmolPath { self.0.as_smol_path() }

	/// Consume into the inner [`SmolPath`].
	pub fn into_smol_path(self) -> SmolPath { self.0.into_smol_path() }
}

impl Deref for WsPath {
	type Target = SmolPath;
	fn deref(&self) -> &SmolPath { self.0.as_smol_path() }
}

impl core::fmt::Display for WsPath {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for WsPath {
	type Err = BevyError;
	fn from_str(val: &str) -> Result<Self> { Self::new(val) }
}

impl TryFrom<&str> for WsPath {
	type Error = BevyError;
	fn try_from(value: &str) -> Result<Self> { Self::new(value) }
}

impl TryFrom<String> for WsPath {
	type Error = BevyError;
	fn try_from(value: String) -> Result<Self> { Self::new(value) }
}

impl TryFrom<SmolStr> for WsPath {
	type Error = BevyError;
	fn try_from(value: SmolStr) -> Result<Self> { Self::new(value) }
}

impl From<FwdPath> for WsPath {
	fn from(value: FwdPath) -> Self { Self(value) }
}

impl From<WsPath> for SmolStr {
	fn from(value: WsPath) -> Self { value.0.into() }
}

impl From<WsPath> for SmolPath {
	fn from(value: WsPath) -> Self { value.0.into() }
}

impl From<&WsPath> for SmolPath {
	fn from(value: &WsPath) -> Self { value.0.clone().into() }
}

impl AsRef<str> for WsPath {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

impl AsRef<SmolPath> for WsPath {
	fn as_ref(&self) -> &SmolPath { self.0.as_smol_path() }
}

// resolving against the workspace root is a std surface
#[cfg(feature = "std")]
mod std_ext {
	use super::WsPath;
	use crate::prelude::*;
	use std::path::Path;

	impl WsPath {
		/// Using calls like `std::fs::read_dir` will return paths relative
		/// to the current directory of the process, not the workspace root.
		/// This function will resolve the difference by first making the path
		/// absolute and then stripping the workspace root.
		pub fn new_cwd_rel(path: impl AsRef<Path>) -> FsResult<Self> {
			AbsPath::new(path)?.into_ws_path()
		}

		/// Convert to an [`AbsPath`]. This should be used instead of
		/// canonicalize/path::absolute because they prepend cwd instead of
		/// workspace root.
		///
		/// ## Panics
		/// Panics if the workspace root or cwd cannot be determined.
		pub fn into_abs(&self) -> AbsPath {
			AbsPath::workspace_root().join(&self.0)
		}
	}

	impl AsRef<Path> for WsPath {
		fn as_ref(&self) -> &Path { Path::new(self.0.as_str()) }
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn cleans_and_refuses_what_leaves() {
		WsPath::new("Cargo.toml").unwrap().as_str().xpect_eq("Cargo.toml");
		WsPath::new("foo/../Cargo.toml")
			.unwrap()
			.as_str()
			.xpect_eq("Cargo.toml");
		WsPath::new("/crates/beet_core").unwrap_err();
		WsPath::new("../elsewhere").unwrap_err();
	}

	#[crate::test]
	#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
	fn resolves_to_abs() {
		WsPath::new(file!()).unwrap().into_abs().xpect_eq(abs_file!());
		WsPath::new_cwd_rel("Cargo.toml")
			.unwrap()
			.as_str()
			.ends_with("Cargo.toml")
			.xpect_true();
	}
}
