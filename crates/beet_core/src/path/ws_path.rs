use crate::prelude::*;
use core::ops::Deref;
use core::str::FromStr;

/// A location relative to the workspace root, usually the directory
/// containing the main `Cargo.toml`: a cleaned [`SmolPath`] with any leading
/// `/` stripped. When used as a component this indicates the entity
/// represents a file or directory with the given path, which does **not**
/// have to exist.
///
/// The type is `no_std`; resolving one to an [`AbsPath`]
/// ([`into_abs`](Self::into_abs)) needs the workspace root and so `std`.
///
/// ## Example
///
/// ```rust
/// # use beet_core::prelude::*;
/// let path = WsPath::new(file!());
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
#[cfg_attr(feature = "serde", serde(from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "tokens", derive(ToTokens), to_tokens(WsPath::new))]
pub struct WsPath(SmolPath);

impl WsPath {
	/// Create a new [`WsPath`], a common use case is to use `file!()`
	/// which is already relative to the workspace root.
	pub fn new(path: impl AsRef<str>) -> Self {
		Self(SmolPath::new(path.as_ref().trim_start_matches('/')))
	}

	/// Append a path below this one, resolving any `..` segments.
	pub fn join(&self, path: impl AsRef<str>) -> Self {
		Self::new(self.0.join(path))
	}

	/// Returns the parent directory, or [`None`] at the workspace root.
	pub fn parent(&self) -> Option<Self> { self.0.parent().map(Self) }

	/// Borrow the inner [`SmolPath`].
	pub fn as_smol_path(&self) -> &SmolPath { &self.0 }

	/// Consume into the inner [`SmolPath`].
	pub fn into_smol_path(self) -> SmolPath { self.0 }
}

impl Deref for WsPath {
	type Target = SmolPath;
	fn deref(&self) -> &SmolPath { &self.0 }
}

impl core::fmt::Display for WsPath {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for WsPath {
	type Err = core::convert::Infallible;
	fn from_str(val: &str) -> Result<Self, Self::Err> { Ok(Self::new(val)) }
}

impl From<&str> for WsPath {
	fn from(value: &str) -> Self { Self::new(value) }
}

impl From<String> for WsPath {
	fn from(value: String) -> Self { Self::new(value) }
}

impl From<SmolStr> for WsPath {
	fn from(value: SmolStr) -> Self { Self::new(value) }
}

impl From<WsPath> for SmolStr {
	fn from(value: WsPath) -> Self { value.0.into_smol_str() }
}

impl From<WsPath> for SmolPath {
	fn from(value: WsPath) -> Self { value.0 }
}

impl From<&WsPath> for SmolPath {
	fn from(value: &WsPath) -> Self { value.0.clone() }
}

impl AsRef<str> for WsPath {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

impl AsRef<SmolPath> for WsPath {
	fn as_ref(&self) -> &SmolPath { &self.0 }
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
	fn cleans_and_strips_root() {
		WsPath::new("Cargo.toml").as_str().xpect_eq("Cargo.toml");
		WsPath::new("foo/../Cargo.toml")
			.as_str()
			.xpect_eq("Cargo.toml");
		WsPath::new("/crates/beet_core")
			.as_str()
			.xpect_eq("crates/beet_core");
	}

	#[crate::test]
	#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
	fn resolves_to_abs() {
		WsPath::new(file!()).into_abs().xpect_eq(abs_file!());
		WsPath::new_cwd_rel("Cargo.toml")
			.unwrap()
			.as_str()
			.ends_with("Cargo.toml")
			.xpect_true();
	}
}
