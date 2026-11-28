use crate::prelude::*;
use core::ops::Deref;
use core::str::FromStr;
#[cfg(feature = "std")]
use std::path::Path;

/// A location within a container, a store, a package or a router, that can
/// never name anything outside it: a cleaned [`SmolPath`] with no leading `/`
/// and no `..` above its start.
///
/// Construction refuses rather than rewrites. A leading `/` is refused since
/// whether it means the container's root or the filesystem's depends on who
/// wrote it, and a `..` climbing above the path's start is refused since it
/// names something outside the container. A `..` that stays within (`a/../b`)
/// is cleaned away, so a [`FwdPath`] holds no `..` at all, and
/// [`join`](Self::join)ing one can never climb out of the path it joins.
///
/// A string known at compile time is written with [`fwd_path!`], checked as
/// it compiles. A url's path is rooted at its router, so it converts through
/// [`Url::fwd_path`], which drops that root and refuses a `..`.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// FwdPath::new("images/hero.png")
/// 	.unwrap()
/// 	.xpect_eq(fwd_path!("images/hero.png"));
/// FwdPath::new("/images/hero.png").unwrap_err();
/// FwdPath::new("../escaped").unwrap_err();
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// serde through the string so a deserialized path holds the invariants
#[cfg_attr(
	feature = "serde",
	serde(try_from = "SmolStr", into = "SmolStr")
)]
#[cfg_attr(feature = "tokens", derive(ToTokens), to_tokens(FwdPath::from_canonical))]
pub struct FwdPath(SmolPath);

impl FwdPath {
	/// Parse a path, cleaning its text and refusing one that is rooted or
	/// climbs above its start.
	///
	/// # Errors
	/// Errors on a leading `/`, or a `..` above the path's start.
	pub fn new(path: impl AsRef<str>) -> Result<Self> {
		let raw = path.as_ref();
		let cleaned = SmolPath::new(raw);
		if cleaned.is_absolute() {
			bevybail!(
				"the path `{raw}` is rooted: a path within its container has no leading `/`"
			);
		}
		if cleaned.segments().first() == Some(&"..") {
			bevybail!(
				"the path `{raw}` climbs out of its container with `..`"
			);
		}
		Self(cleaned).xok()
	}

	/// Whether `path` is already clean, the form [`fwd_path!`] requires of a
	/// literal: no leading or trailing `/`, no empty, `.` or `..` segment and
	/// no `\`. The empty path is the container itself.
	pub const fn is_canonical(path: &str) -> bool {
		let bytes = path.as_bytes();
		if bytes.is_empty() {
			return true;
		}
		let mut start = 0;
		let mut index = 0;
		while index <= bytes.len() {
			if index == bytes.len() || bytes[index] == b'/' {
				let len = index - start;
				if len == 0
					|| (len == 1 && bytes[start] == b'.')
					|| (len == 2
						&& bytes[start] == b'.' && bytes[start + 1] == b'.')
				{
					return false;
				}
				start = index + 1;
			} else if bytes[index] == b'\\' {
				return false;
			}
			index += 1;
		}
		true
	}

	/// Wrap a path already clean, rootless and without a leading `..`, ie
	/// the remainder of a cleaned path after its prefix.
	pub(crate) fn new_unchecked(path: SmolPath) -> Self { Self(path) }

	/// A path from a literal [`is_canonical`](Self::is_canonical) has
	/// checked, see [`fwd_path!`].
	#[doc(hidden)]
	pub const fn from_canonical(path: &'static str) -> Self {
		Self(SmolPath::new_static_unchecked(path))
	}

	/// The path `path` below this one, which can never climb out of it.
	pub fn join(&self, path: &FwdPath) -> Self {
		match (self.is_empty(), path.is_empty()) {
			(_, true) => self.clone(),
			(true, false) => path.clone(),
			(false, false) => Self(SmolPath::new_unchecked(format!(
				"{}/{}",
				self.0, path.0
			))),
		}
	}

	/// The entry `segment` directly below this path.
	pub fn join_segment(&self, segment: &PathSegment) -> Self {
		self.join(&segment.clone().into())
	}

	/// This path's parent and last segment, [`None`] for the empty path.
	pub fn split_last(&self) -> Option<(Self, PathSegment)> {
		let last = self.last_segment()?;
		(
			self.parent().unwrap_or_default(),
			// every segment of a clean path within its container is a plain one
			PathSegment::new_unchecked(last),
		)
			.xsome()
	}

	/// [`join`](Self::join) a path parsed from text, see [`new`](Self::new).
	///
	/// # Errors
	/// Errors when `path` is rooted or climbs above this one.
	pub fn try_join(&self, path: impl AsRef<str>) -> Result<Self> {
		self.join(&Self::new(path)?).xok()
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

	/// A path from segments joined by `/`, see [`new`](Self::new).
	///
	/// # Errors
	/// Errors when the joined path is rooted or climbs above its start.
	pub fn from_segments(segments: &[impl AsRef<str>]) -> Result<Self> {
		Self::new(SmolPath::from_segments(segments))
	}

	/// Borrow the inner [`SmolPath`].
	pub fn as_smol_path(&self) -> &SmolPath { &self.0 }

	/// Consume into the inner [`SmolPath`].
	pub fn into_smol_path(self) -> SmolPath { self.0 }
}

/// A path known at compile time, checked as it compiles: the literal, or the
/// `&'static str` constant, must be canonical, see [`FwdPath::is_canonical`].
///
/// ```
/// # use beet_core::prelude::*;
/// const LOGO: FwdPath = fwd_path!("assets/logo");
/// LOGO.as_str().xpect_eq("assets/logo");
/// ```
///
/// ```compile_fail
/// # use beet_core::prelude::*;
/// fwd_path!("../assets");
/// ```
#[macro_export]
macro_rules! fwd_path {
	($path:expr) => {{
		const _: () = ::core::assert!(
			$crate::prelude::FwdPath::is_canonical($path),
			"a `fwd_path!` path is canonical: no leading or trailing `/`, and no empty, `.` or `..` segment"
		);
		$crate::prelude::FwdPath::from_canonical($path)
	}};
}

impl Deref for FwdPath {
	type Target = SmolPath;
	fn deref(&self) -> &SmolPath { &self.0 }
}

impl core::fmt::Display for FwdPath {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		self.0.fmt(f)
	}
}

impl FromStr for FwdPath {
	type Err = BevyError;
	fn from_str(val: &str) -> Result<Self> { Self::new(val) }
}

impl TryFrom<&str> for FwdPath {
	type Error = BevyError;
	fn try_from(value: &str) -> Result<Self> { Self::new(value) }
}

impl TryFrom<String> for FwdPath {
	type Error = BevyError;
	fn try_from(value: String) -> Result<Self> { Self::new(value) }
}

impl TryFrom<SmolStr> for FwdPath {
	type Error = BevyError;
	fn try_from(value: SmolStr) -> Result<Self> { Self::new(value) }
}

impl TryFrom<SmolPath> for FwdPath {
	type Error = BevyError;
	fn try_from(value: SmolPath) -> Result<Self> { Self::new(value) }
}

#[cfg(feature = "std")]
impl TryFrom<&Path> for FwdPath {
	type Error = BevyError;
	fn try_from(value: &Path) -> Result<Self> {
		Self::new(value.to_string_lossy())
	}
}

/// A cid's text, its multibase prefix and base32, is always one plain
/// segment.
impl From<&Cid> for FwdPath {
	fn from(value: &Cid) -> Self {
		Self(SmolPath::new_unchecked(value.as_str()))
	}
}

impl From<&FwdPath> for FwdPath {
	fn from(value: &FwdPath) -> Self { value.clone() }
}

impl From<FwdPath> for SmolPath {
	fn from(value: FwdPath) -> Self { value.0 }
}

impl From<FwdPath> for SmolStr {
	fn from(value: FwdPath) -> Self { value.0.into_smol_str() }
}

impl AsRef<str> for FwdPath {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

impl AsRef<SmolPath> for FwdPath {
	fn as_ref(&self) -> &SmolPath { &self.0 }
}

#[cfg(feature = "std")]
impl AsRef<Path> for FwdPath {
	fn as_ref(&self) -> &Path { Path::new(self.0.as_str()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn refuses_a_root() {
		FwdPath::new("/a").unwrap_err().to_string().xpect_contains("rooted");
		FwdPath::new("/").unwrap_err();
		FwdPath::new("").unwrap().as_str().xpect_eq("");
	}

	#[crate::test]
	fn refuses_climbing_out() {
		FwdPath::new("../a")
			.unwrap_err()
			.to_string()
			.xpect_contains("climbs out");
		FwdPath::new("a/../..").unwrap_err();
		FwdPath::new("a/../b").unwrap().as_str().xpect_eq("b");
		FwdPath::new("a//b/./").unwrap().as_str().xpect_eq("a/b");
	}

	#[crate::test]
	fn join_stays_within() {
		let dir = fwd_path!("a/b");
		dir.join(&fwd_path!("c")).as_str().xpect_eq("a/b/c");
		dir.join(&FwdPath::default()).xpect_eq(dir.clone());
		FwdPath::default().join(&dir).xpect_eq(dir.clone());
		dir.try_join("../c").unwrap_err();
		dir.try_join("/c").unwrap_err();
	}

	#[crate::test]
	fn checks_a_literal() {
		for valid in ["", "a", "a/b.txt", ".hidden/x", "a..b"] {
			FwdPath::is_canonical(valid).xpect_true();
		}
		for invalid in ["/a", "a/", "a//b", "./a", "a/./b", "../a", "a/..", "a\\b"]
		{
			FwdPath::is_canonical(invalid).xpect_false();
		}
	}

	#[crate::test]
	fn converts_a_cid() {
		let cid = Cid::raw(b"blob");
		fwd_path!("blobs")
			.join(&FwdPath::from(&cid))
			.xpect_eq(FwdPath::new(format!("blobs/{cid}")).unwrap());
	}

	#[crate::test]
	fn derefs_to_smol_path() {
		let path = fwd_path!("dir/file.txt");
		path.extension().xpect_eq(Some("txt"));
		path.parent().unwrap().as_str().xpect_eq("dir");
		path.strip_prefix(&fwd_path!("dir"))
			.unwrap()
			.as_str()
			.xpect_eq("file.txt");
	}

	#[crate::test]
	#[cfg(feature = "json")]
	fn serde_roundtrip() {
		let original = fwd_path!("hello/world");
		MediaBytes::serialize(MediaType::Json, &original)
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("\"hello/world\"");
		MediaType::Json
			.deserialize::<FwdPath>(b"\"hello/world\"")
			.unwrap()
			.xpect_eq(original);
		MediaType::Json
			.deserialize::<FwdPath>(b"\"../hello\"")
			.unwrap_err();
	}
}
