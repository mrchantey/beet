use crate::prelude::*;
use core::ops::Deref;
use core::str::FromStr;
#[cfg(feature = "std")]
use std::path::Path;
#[cfg(feature = "std")]
use std::path::PathBuf;

/// A key within a store: a [`SmolPath`] with no root of its own, so it
/// resolves against whatever root the store supplies (a bucket prefix, a
/// directory, a browser database).
///
/// Construction strips a leading `/` and drops a `..` above the root, so a
/// [`RelPath`] can never name anything outside the store it is a key in; a
/// leading slash or an escaping `..` is unrepresentable rather than silently
/// meaningful. Every other invariant is [`SmolPath`]'s, reached through
/// [`Deref`].
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// RelPath::new("/images/hero.png").xpect_eq(RelPath::new("images/hero.png"));
/// RelPath::new("../escaped").xpect_eq(RelPath::new("escaped"));
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// serde through the string so a deserialized key holds the invariants
#[cfg_attr(feature = "serde", serde(from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "tokens", derive(ToTokens), to_tokens(RelPath::new))]
pub struct RelPath(SmolPath);

impl RelPath {
	/// Create a new [`RelPath`], cleaning the text, stripping any leading `/`
	/// and dropping any `..` above the root.
	pub fn new(path: impl AsRef<str>) -> Self {
		// rooting the text has `clean` drop a `..` above the root, then the
		// root is stripped again
		SmolPath::new(format!("/{}", path.as_ref()))
			.as_str()
			.trim_start_matches('/')
			.xmap(SmolPath::new_unchecked)
			.xmap(Self)
	}

	/// Append another path below this one; an escaping `..` is dropped.
	pub fn join(&self, path: impl AsRef<str>) -> Self {
		Self::new(self.0.join(path))
	}

	/// The path relative to `prefix`, [`None`] when `prefix` is neither this
	/// path nor an ancestor of it.
	pub fn strip_prefix(&self, prefix: &Self) -> Option<Self> {
		self.0.strip_prefix(&prefix.0).map(Self)
	}

	/// The parent path, [`None`] for the empty path.
	pub fn parent(&self) -> Option<Self> { self.0.parent().map(Self) }

	/// Replaces the file extension on the final segment, see
	/// [`SmolPath::with_extension`].
	pub fn with_extension(self, ext: &str) -> Self {
		Self(self.0.with_extension(ext))
	}

	/// Build a [`RelPath`] from segments joined by `/`.
	pub fn from_segments(segments: &[impl AsRef<str>]) -> Self {
		Self::new(SmolPath::from_segments(segments))
	}

	/// Borrow the inner [`SmolPath`].
	pub fn as_smol_path(&self) -> &SmolPath { &self.0 }

	/// Consume into the inner [`SmolPath`].
	pub fn into_smol_path(self) -> SmolPath { self.0 }
}

impl Deref for RelPath {
	type Target = SmolPath;
	fn deref(&self) -> &SmolPath { &self.0 }
}

impl core::fmt::Display for RelPath {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for RelPath {
	type Err = core::convert::Infallible;
	fn from_str(val: &str) -> Result<Self, Self::Err> { Ok(Self::new(val)) }
}

impl From<&str> for RelPath {
	fn from(value: &str) -> Self { Self::new(value) }
}

impl From<String> for RelPath {
	fn from(value: String) -> Self { Self::new(value) }
}

impl From<&String> for RelPath {
	fn from(value: &String) -> Self { Self::new(value) }
}

impl From<SmolStr> for RelPath {
	fn from(value: SmolStr) -> Self { Self::new(value) }
}

impl From<&SmolStr> for RelPath {
	fn from(value: &SmolStr) -> Self { Self::new(value) }
}

impl From<SmolPath> for RelPath {
	fn from(value: SmolPath) -> Self { Self::new(value) }
}

impl From<&SmolPath> for RelPath {
	fn from(value: &SmolPath) -> Self { Self::new(value) }
}

impl From<&RelPath> for RelPath {
	fn from(value: &RelPath) -> Self { value.clone() }
}

impl From<RelPath> for SmolPath {
	fn from(value: RelPath) -> Self { value.0 }
}

impl From<RelPath> for SmolStr {
	fn from(value: RelPath) -> Self { value.0.into_smol_str() }
}

impl From<&Vec<String>> for RelPath {
	fn from(parts: &Vec<String>) -> Self {
		Self::from_segments(parts.as_slice())
	}
}

impl From<&Vec<SmolStr>> for RelPath {
	fn from(parts: &Vec<SmolStr>) -> Self {
		Self::from_segments(parts.as_slice())
	}
}

impl AsRef<str> for RelPath {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

impl AsRef<SmolPath> for RelPath {
	fn as_ref(&self) -> &SmolPath { &self.0 }
}

#[cfg(feature = "std")]
impl AsRef<Path> for RelPath {
	fn as_ref(&self) -> &Path { Path::new(self.0.as_str()) }
}

#[cfg(feature = "std")]
impl From<PathBuf> for RelPath {
	fn from(value: PathBuf) -> Self { Self::new(value.to_string_lossy()) }
}

#[cfg(feature = "std")]
impl From<&Path> for RelPath {
	fn from(value: &Path) -> Self { Self::new(value.to_string_lossy()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn strips_root() {
		RelPath::new("/a").as_str().xpect_eq("a");
		RelPath::new("//a/").as_str().xpect_eq("a");
		RelPath::new("/").as_str().xpect_eq("");
		RelPath::new("").as_str().xpect_eq("");
	}

	#[crate::test]
	fn drops_escaping_parents() {
		RelPath::new("../a").as_str().xpect_eq("a");
		RelPath::new("a/../..").as_str().xpect_eq("");
		RelPath::new("a/../b").as_str().xpect_eq("b");
	}

	#[crate::test]
	fn join_stays_within() {
		RelPath::new("a").join("/b").as_str().xpect_eq("a/b");
		RelPath::new("a").join("../..").as_str().xpect_eq("");
		RelPath::new("a/b").join("../c").as_str().xpect_eq("a/c");
	}

	#[crate::test]
	fn derefs_to_smol_path() {
		let path = RelPath::new("dir/file.txt");
		path.extension().xpect_eq(Some("txt"));
		path.parent().unwrap().as_str().xpect_eq("dir");
		path.strip_prefix(&RelPath::new("dir"))
			.unwrap()
			.as_str()
			.xpect_eq("file.txt");
	}

	#[crate::test]
	#[cfg(feature = "json")]
	fn serde_roundtrip() {
		let original = RelPath::new("hello/world");
		serde_json::to_string(&original)
			.unwrap()
			.xpect_eq("\"hello/world\"");
		let deserialized: RelPath =
			serde_json::from_str("\"/hello/world\"").unwrap();
		original.xpect_eq(deserialized);
	}
}
