use crate::prelude::*;
use core::ops::Deref;
use core::str::FromStr;
#[cfg(feature = "std")]
use std::path::Path;
#[cfg(feature = "std")]
use std::path::PathBuf;

/// A clean, logical `/`-separated path: zero or more segments, optionally
/// terminated by a segment with an extension. Conceptually similar to a URL
/// path component or a stored object key — independent of the filesystem.
///
/// Invariants enforced on construction ([`SmolPath::new`], [`SmolPath::join`],
/// ..):
/// - Never starts or ends with `/`.
/// - Single `/` between segments, no empty segments.
/// - `.` segments are stripped.
/// - `..` collapses the previous segment when one is present.
/// - The empty path is `""` (see [`SmolPath::default`]).
///
/// Backed by [`SmolStr`] for cheap clones and a small inline representation.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let path = SmolPath::new("images/hero.png");
/// let nested = path.join("@2x.webp");
/// nested.to_string();
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct SmolPath(SmolStr);

impl SmolPath {
	/// Create a new [`SmolPath`], stripping leading/trailing `/`s, dropping
	/// empty segments, and collapsing `.`/`..` segments.
	pub fn new(path: impl Into<SmolStr>) -> Self {
		let raw: SmolStr = path.into();
		// `path_ext::clean` keeps a rooted path's leading `/` and may return
		// the `.` placeholder; a [`SmolPath`] is always logically relative.
		let cleaned = path_ext::clean(raw.as_str());
		let cleaned = cleaned.trim_start_matches('/');
		let cleaned = if cleaned == "." { "" } else { cleaned };
		Self(SmolStr::new(cleaned))
	}

	/// Append another path onto this one, returning a new [`SmolPath`]. Any
	/// leading `/` on the joined path is stripped so the result stays
	/// logically relative.
	pub fn join(&self, path: impl AsRef<str>) -> Self {
		let path = path.as_ref().trim_start_matches('/');
		if path.is_empty() {
			self.clone()
		} else if self.0.is_empty() {
			Self::new(path)
		} else {
			let mut combined =
				String::with_capacity(self.0.len() + 1 + path.len());
			combined.push_str(self.0.as_str());
			combined.push('/');
			combined.push_str(path);
			Self::new(combined)
		}
	}

	/// Returns the parent path, or [`None`] if the path is empty.
	pub fn parent(&self) -> Option<Self> {
		if self.0.is_empty() {
			None
		} else {
			match self.0.rfind('/') {
				Some(idx) => Some(Self(SmolStr::new(&self.0[..idx]))),
				None => Some(Self::default()),
			}
		}
	}

	/// Replaces the file extension on the final segment with `ext`. If
	/// `ext` is empty, the existing extension is removed. No-op on an
	/// empty path or a filename composed solely of `.` characters.
	pub fn with_extension(self, ext: &str) -> Self {
		let s = self.0.as_str();
		if s.is_empty() {
			return self;
		}
		let last_slash = s.rfind('/').map(|i| i + 1).unwrap_or(0);
		let filename = &s[last_slash..];
		let stem_len = stem_length(filename);
		if stem_len == 0 {
			return self;
		}
		let stem_end = last_slash + stem_len;
		let mut out = String::with_capacity(stem_end + ext.len() + 1);
		out.push_str(&s[..stem_end]);
		if !ext.is_empty() {
			out.push('.');
			out.push_str(ext);
		}
		Self(SmolStr::new(out))
	}

	/// Build a [`SmolPath`] from segments joined by `/`. Empty segments are
	/// skipped.
	pub fn from_segments(segments: &[impl AsRef<str>]) -> Self {
		let joined = segments
			.iter()
			.map(|seg| seg.as_ref())
			.filter(|seg| !seg.is_empty())
			.collect::<Vec<_>>()
			.join("/");
		Self::new(joined)
	}

	/// Return the `/`-separated segments of the path.
	pub fn segments(&self) -> Vec<&str> {
		if self.0.is_empty() {
			Vec::new()
		} else {
			self.0.split('/').collect()
		}
	}

	/// Returns the first segment, if any.
	pub fn first_segment(&self) -> Option<&str> {
		if self.0.is_empty() {
			None
		} else {
			self.0.split('/').next()
		}
	}

	/// Returns the last segment, if any.
	pub fn last_segment(&self) -> Option<&str> {
		if self.0.is_empty() {
			None
		} else {
			self.0.rsplit('/').next()
		}
	}

	/// The final segment of the path, if any. Alias for
	/// [`SmolPath::last_segment`] mirroring [`std::path::Path::file_name`].
	pub fn file_name(&self) -> Option<&str> { self.last_segment() }

	/// The portion of the final segment before its extension. Matches
	/// [`std::path::Path::file_stem`] semantics.
	pub fn file_stem(&self) -> Option<&str> {
		let filename = self.file_name()?;
		if filename.chars().all(|c| c == '.') {
			Some(filename)
		} else {
			match filename.rfind('.') {
				Some(0) | None => Some(filename),
				Some(idx) => Some(&filename[..idx]),
			}
		}
	}

	/// The extension portion of the final segment, if any. Matches
	/// [`std::path::Path::extension`] semantics.
	pub fn extension(&self) -> Option<&str> {
		let filename = self.file_name()?;
		if filename.chars().all(|c| c == '.') {
			None
		} else {
			match filename.rfind('.') {
				Some(0) | None => None,
				Some(idx) => Some(&filename[idx + 1..]),
			}
		}
	}

	/// The [`MediaType`] inferred from the path's extension, if it has one.
	pub fn media_type(&self) -> Option<MediaType> {
		self.extension().map(MediaType::from_extension)
	}

	/// Return the path prefixed with `/`, useful when a leading slash is
	/// required (eg URL paths). Empty paths return `"/"`.
	pub fn with_leading_slash(&self) -> String {
		if self.0.is_empty() {
			"/".to_string()
		} else {
			format!("/{}", self.0)
		}
	}

	/// Borrow the inner string.
	pub fn as_str(&self) -> &str { self.0.as_str() }

	/// Consume into the inner [`SmolStr`].
	pub fn into_smol_str(self) -> SmolStr { self.0 }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl core::fmt::Display for SmolPath {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(self.0.as_str())
	}
}

impl FromStr for SmolPath {
	type Err = core::convert::Infallible;
	fn from_str(val: &str) -> Result<Self, Self::Err> { Ok(Self::new(val)) }
}

impl From<&str> for SmolPath {
	fn from(value: &str) -> Self { Self::new(value) }
}

impl From<String> for SmolPath {
	fn from(value: String) -> Self { Self::new(value) }
}

impl From<SmolStr> for SmolPath {
	fn from(value: SmolStr) -> Self { Self::new(value) }
}

impl From<&SmolStr> for SmolPath {
	fn from(value: &SmolStr) -> Self { Self::new(value.clone()) }
}

impl From<&Vec<String>> for SmolPath {
	fn from(parts: &Vec<String>) -> Self {
		Self::from_segments(parts.as_slice())
	}
}

impl From<&Vec<SmolStr>> for SmolPath {
	fn from(parts: &Vec<SmolStr>) -> Self {
		Self::from_segments(parts.as_slice())
	}
}

impl Deref for SmolPath {
	type Target = str;
	fn deref(&self) -> &str { self.0.as_str() }
}

impl AsRef<str> for SmolPath {
	fn as_ref(&self) -> &str { self.0.as_str() }
}

// ---------------------------------------------------------------------------
// std interop — kept here (gated) so std call sites can keep using
// `&SmolPath` where a `&Path` was previously accepted.
// ---------------------------------------------------------------------------

#[cfg(feature = "std")]
impl SmolPath {
	/// Return the path as a [`PathBuf`].
	pub fn to_path_buf(&self) -> PathBuf { PathBuf::from(self.0.as_str()) }
}

#[cfg(feature = "std")]
impl AsRef<Path> for SmolPath {
	fn as_ref(&self) -> &Path { Path::new(self.0.as_str()) }
}

#[cfg(feature = "std")]
impl From<PathBuf> for SmolPath {
	fn from(value: PathBuf) -> Self {
		Self::new(value.to_string_lossy().into_owned())
	}
}

#[cfg(feature = "std")]
impl From<&Path> for SmolPath {
	fn from(value: &Path) -> Self {
		Self::new(value.to_string_lossy().into_owned())
	}
}

#[cfg(feature = "std")]
impl From<SmolPath> for PathBuf {
	fn from(value: SmolPath) -> Self { PathBuf::from(value.0.as_str()) }
}

/// Length of the stem portion of `filename`, matching
/// [`std::path::Path::file_stem`] semantics — used by `with_extension`.
fn stem_length(filename: &str) -> usize {
	if filename.is_empty() {
		return 0;
	}
	if filename.chars().all(|c| c == '.') {
		return filename.len();
	}
	match filename.rfind('.') {
		Some(0) | None => filename.len(),
		Some(idx) => idx,
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn strips_leading_slash() {
		SmolPath::new("/hello").to_string().xpect_eq("hello");
		SmolPath::new("hello").to_string().xpect_eq("hello");
	}

	#[crate::test]
	fn strips_trailing_slash() {
		SmolPath::new("hello/").to_string().xpect_eq("hello");
		SmolPath::new("/hello/").to_string().xpect_eq("hello");
	}

	#[crate::test]
	fn default_is_empty() {
		SmolPath::default().to_string().xpect_eq("");
		SmolPath::new("/").to_string().xpect_eq("");
	}

	#[crate::test]
	fn cleans_path() {
		SmolPath::new("foo/bar/../baz")
			.to_string()
			.xpect_eq("foo/baz");
	}

	#[crate::test]
	fn cleans_repeated_slashes() {
		SmolPath::new("foo//bar///baz")
			.to_string()
			.xpect_eq("foo/bar/baz");
	}

	#[crate::test]
	fn preserves_leading_parents() {
		SmolPath::new("../foo").to_string().xpect_eq("../foo");
		SmolPath::new("foo/../..").to_string().xpect_eq("..");
	}

	#[crate::test]
	fn join_works() {
		SmolPath::new("foo")
			.join("bar")
			.to_string()
			.xpect_eq("foo/bar");
	}

	#[crate::test]
	fn join_strips_leading_slash() {
		SmolPath::new("foo")
			.join("/bar")
			.to_string()
			.xpect_eq("foo/bar");
	}

	#[crate::test]
	fn join_empty_is_identity() {
		SmolPath::new("foo").join("").to_string().xpect_eq("foo");
	}

	#[crate::test]
	fn join_smol_path_root_is_identity() {
		SmolPath::new("foo")
			.join(&SmolPath::new("/"))
			.to_string()
			.xpect_eq("foo");
	}

	#[crate::test]
	fn parent() {
		SmolPath::new("foo/bar")
			.parent()
			.unwrap()
			.to_string()
			.xpect_eq("foo");
		SmolPath::new("foo")
			.parent()
			.unwrap()
			.to_string()
			.xpect_eq("");
		SmolPath::default().parent().xpect_none();
	}

	#[crate::test]
	fn with_extension() {
		SmolPath::new("foo/bar")
			.with_extension("txt")
			.to_string()
			.xpect_eq("foo/bar.txt");
		SmolPath::new("foo/bar.baz")
			.with_extension("txt")
			.to_string()
			.xpect_eq("foo/bar.txt");
		SmolPath::new("foo/bar.baz")
			.with_extension("")
			.to_string()
			.xpect_eq("foo/bar");
	}

	#[crate::test]
	fn from_segments() {
		let segments = vec!["api", "users", "123"];
		SmolPath::from_segments(&segments)
			.to_string()
			.xpect_eq("api/users/123");
	}

	#[crate::test]
	fn from_segments_empty() {
		let segments: Vec<&str> = vec![];
		SmolPath::from_segments(&segments).to_string().xpect_eq("");
	}

	#[crate::test]
	fn segments() {
		let path = SmolPath::new("api/users/123");
		path.segments().xpect_eq(vec!["api", "users", "123"]);
		SmolPath::default().segments().xpect_eq(Vec::<&str>::new());
	}

	#[crate::test]
	fn first_last_segment() {
		let path = SmolPath::new("api/users/123");
		path.first_segment().unwrap().xpect_eq("api");
		path.last_segment().unwrap().xpect_eq("123");

		let empty_path = SmolPath::default();
		empty_path.first_segment().xpect_none();
		empty_path.last_segment().xpect_none();
	}

	#[crate::test]
	fn with_leading_slash() {
		SmolPath::new("foo/bar")
			.with_leading_slash()
			.xpect_eq("/foo/bar");
		SmolPath::default().with_leading_slash().xpect_eq("/");
	}

	#[crate::test]
	fn extension_and_stem() {
		let path = SmolPath::new("dir/file.tar.gz");
		path.extension().xpect_eq(Some("gz"));
		path.file_stem().xpect_eq(Some("file.tar"));

		let no_ext = SmolPath::new("foo/bar");
		no_ext.extension().xpect_none();
		no_ext.file_stem().xpect_eq(Some("bar"));

		let dotfile = SmolPath::new(".env");
		dotfile.extension().xpect_none();
		dotfile.file_stem().xpect_eq(Some(".env"));
	}

	#[crate::test]
	fn display() { SmolPath::new("a/b/c").to_string().xpect_eq("a/b/c"); }

	#[crate::test]
	#[cfg(feature = "json")]
	fn serde_roundtrip() {
		let original = SmolPath::new("hello/world");
		let serialized = serde_json::to_string(&original).unwrap();
		let deserialized: SmolPath = serde_json::from_str(&serialized).unwrap();
		original.xpect_eq(deserialized);
	}
}
