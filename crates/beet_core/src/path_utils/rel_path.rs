use crate::prelude::*;
use path_clean::PathClean;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

/// A relative path whose base can change at runtime.
///
/// Unlike [`AbsPathBuf`] or [`WsPathBuf`], a [`RelPath`] has no fixed root.
/// The same relative path can resolve against an S3 bucket, a parent directory,
/// a URL origin, etc. Multiple [`RelPath`] values can be joined together to
/// form longer relative paths.
///
/// Leading slashes are stripped on construction so the path is always
/// genuinely relative. The inner [`PathBuf`] is cleaned via [`path_clean`].
///
/// ## Example
///
/// ```rust
/// # use beet_core::prelude::*;
/// let path = RelPath::new("images/hero.png");
/// let nested = path.join("@2x.webp");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RelPath(PathBuf);

impl Default for RelPath {
	fn default() -> Self { Self(PathBuf::new()) }
}

impl std::fmt::Display for RelPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0.to_string_lossy())
	}
}

impl RelPath {
	/// Create a new [`RelPath`], stripping any leading `/` and cleaning
	/// the result with [`path_clean`].
	pub fn new(path: impl Into<PathBuf>) -> Self {
		let raw = path.into();
		let lossy = raw.to_string_lossy();
		let trimmed = lossy.trim_start_matches('/');
		if trimmed.is_empty() {
			Self::default()
		} else {
			Self(PathBuf::from(trimmed).clean())
		}
	}

	/// Join another path onto this one, producing a new [`RelPath`].
	/// Any leading `/` on the joined path is stripped so the result
	/// stays relative.
	pub fn join(&self, path: impl AsRef<Path>) -> Self {
		let path = path.as_ref();
		let path = path.strip_prefix("/").unwrap_or(path);
		if path == Path::new("") {
			self.clone()
		} else {
			Self(self.0.join(path).clean())
		}
	}

	/// Returns the parent directory, or [`None`] if at the root.
	pub fn parent(&self) -> Option<Self> {
		self.0.parent().and_then(|parent| {
			if parent == Path::new("") && self.0 == PathBuf::from("") {
				None
			} else {
				Some(Self::new(parent))
			}
		})
	}

	/// Replaces the file extension with the given value.
	pub fn with_extension(mut self, ext: &str) -> Self {
		self.0.set_extension(ext);
		self
	}

	/// Consume the wrapper and return the inner [`PathBuf`].
	pub fn take(self) -> PathBuf { self.0 }

	/// Returns the inner [`PathBuf`] reference.
	pub fn inner(&self) -> &Path { &self.0 }

	/// Build a [`RelPath`] from an iterator of segments joined by `/`.
	pub fn from_segments(segments: &[impl AsRef<str>]) -> Self {
		let joined: String = segments
			.iter()
			.map(|seg| seg.as_ref())
			.filter(|seg| !seg.is_empty())
			.collect::<Vec<_>>()
			.join("/");
		Self::new(joined)
	}

	/// Return the `/`-separated segments of the path.
	pub fn segments(&self) -> Vec<&str> {
		self.0
			.to_str()
			.unwrap_or_default()
			.split('/')
			.filter(|segment| !segment.is_empty())
			.collect()
	}

	/// Returns the first segment, if any.
	pub fn first_segment(&self) -> Option<&str> {
		self.segments().first().copied()
	}

	/// Returns the last segment, if any.
	pub fn last_segment(&self) -> Option<&str> {
		self.segments().last().copied()
	}

	/// Return the path prefixed with `/`, useful when a leading slash is
	/// required (eg URL paths).
	pub fn with_leading_slash(&self) -> String {
		let display = self.0.to_string_lossy();
		if display.is_empty() {
			"/".to_string()
		} else {
			format!("/{display}")
		}
	}
}

// ---------------------------------------------------------------------------
// Trait implementations — kept symmetrical with AbsPathBuf / WsPathBuf
// ---------------------------------------------------------------------------

impl From<String> for RelPath {
	fn from(value: String) -> Self { Self::new(value) }
}

impl From<&str> for RelPath {
	fn from(value: &str) -> Self { Self::new(value) }
}
impl From<&Vec<String>> for RelPath {
	fn from(parts: &Vec<String>) -> Self { Self::new(parts.join("/")) }
}

impl From<PathBuf> for RelPath {
	fn from(value: PathBuf) -> Self { Self::new(value) }
}

impl Into<PathBuf> for RelPath {
	fn into(self) -> PathBuf { self.0 }
}

impl FromStr for RelPath {
	type Err = core::convert::Infallible;
	fn from_str(val: &str) -> Result<Self, Self::Err> { Ok(Self::new(val)) }
}

impl std::ops::Deref for RelPath {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl std::ops::DerefMut for RelPath {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl AsRef<Path> for RelPath {
	fn as_ref(&self) -> &Path { self.0.as_path() }
}

impl AsRef<str> for RelPath {
	fn as_ref(&self) -> &str { self.0.to_str().unwrap_or_default() }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn strips_leading_slash() {
		RelPath::new("/hello").to_string().xpect_eq("hello");
		RelPath::new("hello").to_string().xpect_eq("hello");
	}

	#[test]
	fn default_is_empty() {
		RelPath::default().to_string().xpect_eq("");
		RelPath::new("/").to_string().xpect_eq("");
	}

	#[test]
	fn cleans_path() {
		RelPath::new("foo/bar/../baz")
			.to_string()
			.xpect_eq("foo/baz");
	}

	#[test]
	fn join_works() {
		RelPath::new("foo")
			.join("bar")
			.to_string()
			.xpect_eq("foo/bar");
	}

	#[test]
	fn join_strips_leading_slash() {
		RelPath::new("foo")
			.join("/bar")
			.to_string()
			.xpect_eq("foo/bar");
	}

	#[test]
	fn join_empty_is_identity() {
		RelPath::new("foo").join("").to_string().xpect_eq("foo");
	}

	#[test]
	fn parent() {
		RelPath::new("foo/bar")
			.parent()
			.unwrap()
			.to_string()
			.xpect_eq("foo");
	}

	#[test]
	fn with_extension() {
		RelPath::new("foo/bar")
			.with_extension("txt")
			.to_string()
			.xpect_eq("foo/bar.txt");
	}

	#[test]
	fn from_segments() {
		let segments = vec!["api", "users", "123"];
		RelPath::from_segments(&segments)
			.to_string()
			.xpect_eq("api/users/123");
	}

	#[test]
	fn from_segments_empty() {
		let segments: Vec<&str> = vec![];
		RelPath::from_segments(&segments).to_string().xpect_eq("");
	}

	#[test]
	fn segments() {
		let path = RelPath::new("api/users/123");
		path.segments().xpect_eq(vec!["api", "users", "123"]);
	}

	#[test]
	fn first_last_segment() {
		let path = RelPath::new("api/users/123");
		path.first_segment().unwrap().xpect_eq("api");
		path.last_segment().unwrap().xpect_eq("123");

		let empty_path = RelPath::default();
		empty_path.first_segment().xpect_none();
		empty_path.last_segment().xpect_none();
	}

	#[test]
	fn with_leading_slash() {
		RelPath::new("foo/bar")
			.with_leading_slash()
			.xpect_eq("/foo/bar");
		RelPath::default().with_leading_slash().xpect_eq("/");
	}

	#[test]
	fn display() { RelPath::new("a/b/c").to_string().xpect_eq("a/b/c"); }

	#[test]
	#[cfg(feature = "json")]
	fn serde_roundtrip() {
		let original = RelPath::new("hello/world");
		let serialized = serde_json::to_string(&original).unwrap();
		let deserialized: RelPath = serde_json::from_str(&serialized).unwrap();
		original.xpect_eq(deserialized);
	}
}
