use crate::prelude::*;
use std::path::Path;
use std::path::PathBuf;

/// Describes an absolute path to a route, beginning with `/`.
///
/// This type represents the path portion of a URL or CLI command:
///
/// ```txt
/// https://example.com/foo/bar.txt
///                    ^^^^^^^^^^^^
/// ```
///
/// For CLI commands, this represents the joined positional arguments:
///
/// ```txt
/// myapp users list --verbose
///       ^^^^^^^^^^
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RoutePath(pub PathBuf);

impl Default for RoutePath {
	fn default() -> Self { Self(PathBuf::from("/")) }
}

/// Routes shouldn't have OS-specific paths
/// so we allow to_string
impl std::fmt::Display for RoutePath {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{}", self.0.display())
	}
}

/// Extract the route path from request metadata
impl FromRequestMeta<Self> for RoutePath {
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		Self::new(req.path_string()).xok()
	}
}

impl From<String> for RoutePath {
	fn from(value: String) -> Self { Self::new(value) }
}

impl From<&str> for RoutePath {
	fn from(value: &str) -> Self { Self::new(value) }
}

impl From<PathBuf> for RoutePath {
	fn from(value: PathBuf) -> Self { Self::new(value) }
}

impl Into<PathBuf> for RoutePath {
	fn into(self) -> PathBuf { self.0 }
}

impl std::ops::Deref for RoutePath {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl AsRef<Path> for RoutePath {
	fn as_ref(&self) -> &Path { self.0.as_path() }
}

impl AsRef<str> for RoutePath {
	fn as_ref(&self) -> &str { self.0.to_str().unwrap_or_default() }
}

impl From<RoutePath> for Request {
	fn from(value: RoutePath) -> Request {
		Request::new(HttpMethod::Get, value)
	}
}
#[cfg(feature = "http")]
impl TryInto<http::Uri> for RoutePath {
	type Error = http::uri::InvalidUri;

	fn try_into(self) -> Result<http::Uri, Self::Error> {
		http::Uri::try_from(self.0.to_string_lossy().as_ref())
	}
}

impl RoutePath {
	/// Creates a new [`RoutePath`] from a string, ensuring it starts with `/`
	pub fn new(path: impl Into<PathBuf>) -> Self {
		let path_buf = path.into();
		let path_str = path_buf.to_string_lossy();
		if path_str.starts_with('/') {
			Self(path_buf)
		} else {
			Self(PathBuf::from(format!("/{}", path_str)))
		}
	}

	/// Creates a RoutePath from path segments
	pub fn from_segments(segments: &[String]) -> Self {
		if segments.is_empty() {
			Self::default()
		} else {
			Self(PathBuf::from(format!("/{}", segments.join("/"))))
		}
	}

	/// Creates a RoutePath from RequestParts
	pub fn from_parts(parts: &RequestParts) -> Self {
		Self::from_segments(parts.path())
	}

	/// When joining with other paths ensure that the path
	/// does not start with a leading slash, as this would
	/// cause the path to be treated as an absolute path
	pub fn as_relative(&self) -> &Path {
		self.0.strip_prefix("/").unwrap_or(&self.0)
	}

	/// Creates a route join even if the other route path begins with `/`
	pub fn join(&self, new_path: impl AsRef<Path>) -> Self {
		let new_path = new_path.as_ref();
		let new_path = new_path.strip_prefix("/").unwrap_or(new_path);
		if new_path == Path::new("") {
			self.clone()
		} else {
			Self(self.0.join(&new_path))
		}
	}

	/// Returns the inner PathBuf reference
	pub fn inner(&self) -> &Path { &self.0 }

	/// Given a local path, return a new [`RoutePath`] with:
	/// - any extension removed
	/// - 'index' file stems removed
	/// - leading `/` added if not present
	///
	/// Backslashes are not transformed
	pub fn from_file_path(file_path: impl AsRef<Path>) -> Result<Self> {
		let mut path = file_path.as_ref().to_path_buf();
		path.set_extension("");
		if path
			.file_stem()
			.map(|stem| stem.to_str().map(|stem| stem == "index"))
			.flatten()
			.unwrap_or_default()
		{
			path.pop();
		}
		let mut raw_str = path.to_string_lossy().to_string();
		if raw_str.len() > 1 && raw_str.ends_with('/') {
			raw_str.pop();
		}
		if !raw_str.starts_with('/') {
			raw_str = format!("/{}", raw_str);
		}

		Ok(Self(PathBuf::from(raw_str)))
	}

	/// Returns the path segments as a slice
	pub fn segments(&self) -> Vec<&str> {
		self.0
			.to_str()
			.unwrap_or_default()
			.split('/')
			.filter(|segment| !segment.is_empty())
			.collect()
	}

	/// Returns the first segment of the path, if any
	pub fn first_segment(&self) -> Option<&str> {
		self.segments().first().copied()
	}

	/// Returns the last segment of the path, if any
	pub fn last_segment(&self) -> Option<&str> {
		self.segments().last().copied()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn route_path() {
		RoutePath::new("hello").to_string().xpect_eq("/hello");

		for (value, expected) in [
			("hello", "/hello"),
			("hello.rs", "/hello"),
			("hello/index.rs", "/hello"),
			("/hello/index/", "/hello"),
			("/hello/index.rs/", "/hello"),
			("/hello/index.rs", "/hello"),
			("/index.rs", "/"),
			("/index.rs/", "/"),
			("/index/hi", "/index/hi"),
			("/index/hi/", "/index/hi"),
		] {
			RoutePath::from_file_path(value)
				.unwrap()
				.to_string()
				.xpect_eq(expected);
		}
	}

	#[test]
	fn join() {
		RoutePath::new("/foo")
			.join(&RoutePath::new("/"))
			.to_string()
			.xpect_eq("/foo");
	}

	#[test]
	fn from_segments() {
		let segments =
			vec!["api".to_string(), "users".to_string(), "123".to_string()];
		let path = RoutePath::from_segments(&segments);
		path.to_string().xpect_eq("/api/users/123");
	}

	#[test]
	fn from_segments_empty() {
		let path = RoutePath::from_segments(&[]);
		path.to_string().xpect_eq("/");
	}

	#[test]
	fn segments() {
		let path = RoutePath::new("/api/users/123");
		path.segments().xpect_eq(vec!["api", "users", "123"]);
	}

	#[test]
	fn first_last_segment() {
		let path = RoutePath::new("/api/users/123");
		path.first_segment().unwrap().xpect_eq("api");
		path.last_segment().unwrap().xpect_eq("123");

		let empty_path = RoutePath::default();
		empty_path.first_segment().xpect_none();
		empty_path.last_segment().xpect_none();
	}

	#[test]
	fn from_request_parts() {
		let parts = RequestParts::get("/api/users/123");
		let path = RoutePath::from_parts(&parts);
		path.to_string().xpect_eq("/api/users/123");
	}
}
