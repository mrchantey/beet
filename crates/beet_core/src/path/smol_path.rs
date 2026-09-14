use crate::prelude::*;
use core::ops::Deref;
use core::str::FromStr;
#[cfg(feature = "std")]
use std::path::Path;
#[cfg(feature = "std")]
use std::path::PathBuf;

/// A clean, `/`-separated path string, independent of the filesystem: the
/// unopinionated base every path type in beet is built on.
///
/// Construction normalises through [`path_ext::clean`] and nothing more, so a
/// [`SmolPath`] carries whatever rootedness its text had:
/// - repeated slashes and `.` segments collapse, no trailing `/`
/// - `..` resolves against a preceding named segment
/// - a leading `/` is preserved ([`is_absolute`](Self::is_absolute))
/// - a leading `..` on a relative path is kept
/// - the empty path is `""` (see [`SmolPath::default`])
///
/// A path with a known relationship to a root is one of the opinionated
/// newtypes over it: a [`RelPath`] is a key within a store, an [`AbsPath`] a
/// filesystem location, a [`WsPath`] a location relative to the workspace.
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
// serde through the string so a deserialized path is cleaned like a constructed one
#[cfg_attr(feature = "serde", serde(from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "tokens", derive(ToTokens), to_tokens(SmolPath::new))]
pub struct SmolPath(SmolStr);

impl SmolPath {
	/// Create a new [`SmolPath`], cleaning the text with [`path_ext::clean`].
	/// On windows `\` separators are normalised to `/` first.
	pub fn new(path: impl AsRef<str>) -> Self {
		let raw = path.as_ref();
		#[cfg(target_os = "windows")]
		let raw = &raw.replace('\\', "/");
		let cleaned = path_ext::clean(raw);
		// `clean` returns the `.` placeholder for an empty relative result
		match cleaned.as_str() {
			"." => Self::default(),
			cleaned => Self(SmolStr::new(cleaned)),
		}
	}

	/// Wrap text that is already [`path_ext::clean`] output.
	pub(crate) fn new_unchecked(path: impl Into<SmolStr>) -> Self {
		Self(path.into())
	}

	/// Whether the path is rooted, ie starts with `/`.
	pub fn is_absolute(&self) -> bool { self.0.starts_with('/') }

	/// Append another path below this one, returning a new [`SmolPath`]. The
	/// join is always *below*: a leading `/` on `path` does not replace this
	/// path as [`std::path::Path::join`] would, and `..` segments resolve.
	pub fn join(&self, path: impl AsRef<str>) -> Self {
		let path = path.as_ref();
		if path.is_empty() {
			self.clone()
		} else if self.0.is_empty() {
			Self::new(path)
		} else {
			Self::new(format!("{}/{path}", self.0))
		}
	}

	/// The path relative to `prefix`, [`None`] when `prefix` is neither this
	/// path nor an ancestor of it. Segment-aware: `"foo/bar"` is not under
	/// `"fo"`. An empty `prefix` yields the path unchanged.
	pub fn strip_prefix(&self, prefix: &Self) -> Option<Self> {
		if prefix.0.is_empty() {
			return Some(self.clone());
		}
		let rest = self.0.strip_prefix(prefix.0.as_str())?;
		match rest.strip_prefix('/') {
			Some(rest) => Some(Self::new(rest)),
			// the exact prefix, or the root `/` prefix which ends in its own separator
			None if rest.is_empty() || prefix.0.ends_with('/') => {
				Some(Self::new(rest))
			}
			None => None,
		}
	}

	/// The parent path: [`None`] for the empty path and the root `/`, `""`
	/// for a single relative segment, `/` for a single rooted one.
	pub fn parent(&self) -> Option<Self> {
		match self.0.as_str() {
			"" | "/" => None,
			path => match path.rfind('/') {
				Some(0) => Some(Self(SmolStr::new_static("/"))),
				Some(idx) => Some(Self(SmolStr::new(&path[..idx]))),
				None => Some(Self::default()),
			},
		}
	}

	/// Replaces the file extension on the final segment with `ext`. If
	/// `ext` is empty, the existing extension is removed. No-op on an
	/// empty path or a filename composed solely of `.` characters.
	pub fn with_extension(self, ext: &str) -> Self {
		let path = self.0.as_str();
		let last_slash = path.rfind('/').map(|i| i + 1).unwrap_or(0);
		let stem_len = stem_length(&path[last_slash..]);
		if stem_len == 0 {
			return self;
		}
		let stem_end = last_slash + stem_len;
		let mut out = String::with_capacity(stem_end + ext.len() + 1);
		out.push_str(&path[..stem_end]);
		if !ext.is_empty() {
			out.push('.');
			out.push_str(ext);
		}
		Self(SmolStr::new(out))
	}

	/// Build a [`SmolPath`] from segments joined by `/`. Empty segments are
	/// skipped.
	pub fn from_segments(segments: &[impl AsRef<str>]) -> Self {
		segments
			.iter()
			.map(|seg| seg.as_ref())
			.filter(|seg| !seg.is_empty())
			.collect::<Vec<_>>()
			.join("/")
			.xmap(Self::new)
	}

	/// The named segments of the path, excluding the root.
	pub fn segments(&self) -> Vec<&str> {
		self.0.split('/').filter(|seg| !seg.is_empty()).collect()
	}

	/// Returns the first named segment, if any.
	pub fn first_segment(&self) -> Option<&str> {
		self.0.split('/').find(|seg| !seg.is_empty())
	}

	/// Returns the last named segment, if any.
	pub fn last_segment(&self) -> Option<&str> {
		self.0.rsplit('/').find(|seg| !seg.is_empty())
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
	/// required (eg URL paths). Empty paths return `"/"`, a rooted path is
	/// returned as is.
	pub fn with_leading_slash(&self) -> String {
		match self.0.as_str() {
			"" => "/".to_string(),
			path if path.starts_with('/') => path.to_string(),
			path => format!("/{path}"),
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

impl From<&String> for SmolPath {
	fn from(value: &String) -> Self { Self::new(value) }
}

impl From<SmolStr> for SmolPath {
	fn from(value: SmolStr) -> Self { Self::new(value) }
}

impl From<&SmolStr> for SmolPath {
	fn from(value: &SmolStr) -> Self { Self::new(value) }
}

impl From<&SmolPath> for SmolPath {
	fn from(value: &SmolPath) -> Self { value.clone() }
}

impl From<SmolPath> for SmolStr {
	fn from(value: SmolPath) -> Self { value.0 }
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

impl AsRef<SmolPath> for SmolPath {
	fn as_ref(&self) -> &SmolPath { self }
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
	fn from(value: PathBuf) -> Self { Self::new(value.to_string_lossy()) }
}

#[cfg(feature = "std")]
impl From<&PathBuf> for SmolPath {
	fn from(value: &PathBuf) -> Self { Self::new(value.to_string_lossy()) }
}

#[cfg(feature = "std")]
impl From<&Path> for SmolPath {
	fn from(value: &Path) -> Self { Self::new(value.to_string_lossy()) }
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
	fn keeps_leading_slash() {
		SmolPath::new("/hello").to_string().xpect_eq("/hello");
		SmolPath::new("hello").to_string().xpect_eq("hello");
		SmolPath::new("/hello").is_absolute().xpect_true();
		SmolPath::new("hello").is_absolute().xpect_false();
	}

	#[crate::test]
	fn strips_trailing_slash() {
		SmolPath::new("hello/").to_string().xpect_eq("hello");
		SmolPath::new("/hello/").to_string().xpect_eq("/hello");
	}

	#[crate::test]
	fn default_is_empty() {
		SmolPath::default().to_string().xpect_eq("");
		SmolPath::new("").to_string().xpect_eq("");
		SmolPath::new(".").to_string().xpect_eq("");
		SmolPath::new("/").to_string().xpect_eq("/");
	}

	#[crate::test]
	fn cleans_path() {
		SmolPath::new("foo/bar/../baz")
			.to_string()
			.xpect_eq("foo/baz");
		SmolPath::new("/foo/../..").to_string().xpect_eq("/");
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
		SmolPath::new("/foo")
			.join("bar")
			.to_string()
			.xpect_eq("/foo/bar");
		SmolPath::new("/").join("bar").to_string().xpect_eq("/bar");
		SmolPath::default()
			.join("/bar")
			.to_string()
			.xpect_eq("/bar");
	}

	#[crate::test]
	fn join_is_always_below() {
		SmolPath::new("foo")
			.join("/bar")
			.to_string()
			.xpect_eq("foo/bar");
		SmolPath::new("/foo")
			.join("/bar")
			.to_string()
			.xpect_eq("/foo/bar");
	}

	#[crate::test]
	fn join_resolves_parents() {
		SmolPath::new("foo/bar")
			.join("../baz")
			.to_string()
			.xpect_eq("foo/baz");
		SmolPath::new("foo")
			.join("../..")
			.to_string()
			.xpect_eq("..");
		SmolPath::new("/foo")
			.join("../..")
			.to_string()
			.xpect_eq("/");
	}

	#[crate::test]
	fn join_empty_is_identity() {
		SmolPath::new("foo").join("").to_string().xpect_eq("foo");
		SmolPath::new("foo")
			.join(&SmolPath::new("/"))
			.to_string()
			.xpect_eq("foo");
	}

	#[crate::test]
	fn strip_prefix() {
		SmolPath::new("a/b/c.txt")
			.strip_prefix(&SmolPath::new("a"))
			.xpect_eq(Some(SmolPath::new("b/c.txt")));
		SmolPath::new("a/b")
			.strip_prefix(&SmolPath::new("a/b"))
			.xpect_eq(Some(SmolPath::default()));
		SmolPath::new("a/b")
			.strip_prefix(&SmolPath::default())
			.xpect_eq(Some(SmolPath::new("a/b")));
		SmolPath::new("/a/b")
			.strip_prefix(&SmolPath::new("/a"))
			.xpect_eq(Some(SmolPath::new("b")));
		SmolPath::new("/a/b")
			.strip_prefix(&SmolPath::new("/"))
			.xpect_eq(Some(SmolPath::new("a/b")));
		// segment-aware: `ab` is not under `a`
		SmolPath::new("ab/c")
			.strip_prefix(&SmolPath::new("a"))
			.xpect_none();
		SmolPath::new("a")
			.strip_prefix(&SmolPath::new("a/b"))
			.xpect_none();
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
		SmolPath::new("/foo")
			.parent()
			.unwrap()
			.to_string()
			.xpect_eq("/");
		SmolPath::new("/").parent().xpect_none();
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
		SmolPath::new("/")
			.with_extension("txt")
			.to_string()
			.xpect_eq("/");
	}

	#[crate::test]
	fn from_segments() {
		let segments = vec!["api", "users", "123"];
		SmolPath::from_segments(&segments)
			.to_string()
			.xpect_eq("api/users/123");
		let segments: Vec<&str> = vec![];
		SmolPath::from_segments(&segments).to_string().xpect_eq("");
	}

	#[crate::test]
	fn segments() {
		let path = SmolPath::new("api/users/123");
		path.segments().xpect_eq(vec!["api", "users", "123"]);
		SmolPath::new("/api/users")
			.segments()
			.xpect_eq(vec!["api", "users"]);
		SmolPath::new("/").segments().xpect_eq(Vec::<&str>::new());
		SmolPath::default().segments().xpect_eq(Vec::<&str>::new());
	}

	#[crate::test]
	fn first_last_segment() {
		let path = SmolPath::new("api/users/123");
		path.first_segment().unwrap().xpect_eq("api");
		path.last_segment().unwrap().xpect_eq("123");
		SmolPath::new("/api")
			.first_segment()
			.unwrap()
			.xpect_eq("api");
		SmolPath::new("/").first_segment().xpect_none();

		let empty_path = SmolPath::default();
		empty_path.first_segment().xpect_none();
		empty_path.last_segment().xpect_none();
	}

	#[crate::test]
	fn with_leading_slash() {
		SmolPath::new("foo/bar")
			.with_leading_slash()
			.xpect_eq("/foo/bar");
		SmolPath::new("/foo/bar")
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
		let original = SmolPath::new("/hello/world");
		let serialized = serde_json::to_string(&original).unwrap();
		serialized.xpect_eq("\"/hello/world\"");
		let deserialized: SmolPath = serde_json::from_str(&serialized).unwrap();
		original.xpect_eq(deserialized);
		// a deserialized path is cleaned
		serde_json::from_str::<SmolPath>("\"a//b/./c\"")
			.unwrap()
			.xpect_eq(SmolPath::new("a/b/c"));
	}
}
