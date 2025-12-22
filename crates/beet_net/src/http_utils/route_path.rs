use crate::prelude::*;
use beet_core::prelude::*;
use http::Uri;
use http::uri::InvalidUri;
use std::path::Path;
use std::path::PathBuf;


/// Describes an absolute path to a route, beginning with `/`.
///
/// ```txt
/// https://example.com/foo/bar.txt
///                    ^^^^^^^^^^^^
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct RoutePath(pub PathBuf);

impl Default for RoutePath {
	fn default() -> Self { Self(PathBuf::from("/")) }
}


/// routes shouldnt have os specific paths
/// so we allow to_string
impl std::fmt::Display for RoutePath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0.display())
	}
}

/// The uri path of the request, if the request has no leading slash
/// this will be empty.
impl FromRequestMeta<Self> for RoutePath {
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		let path = req.uri.path();
		Self::new(path).xok()
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

impl Into<Request> for RoutePath {
	fn into(self) -> Request { Request::new(HttpMethod::Get, self) }
}

impl TryInto<Uri> for RoutePath {
	type Error = InvalidUri;

	fn try_into(self) -> Result<Uri, Self::Error> {
		Uri::try_from(self.0.to_string_lossy().as_ref())
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


	/// when joining with other paths ensure that the path
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
	pub fn inner(&self) -> &Path { &self.0 }
	/// given a local path, return a new [`RoutePath`] with:
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
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;



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
}
