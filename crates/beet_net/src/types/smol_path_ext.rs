use super::*;
use beet_core::prelude::*;

/// Extract the relative path from request metadata.
impl FromRequestMeta<Self> for SmolPath {
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		Self::new(req.path_string()).xok()
	}
}

impl From<SmolPath> for Url {
	fn from(value: SmolPath) -> Url {
		let path_str: &str = value.as_ref();
		Url::parse(path_str)
	}
}

impl From<SmolPath> for Request {
	fn from(value: SmolPath) -> Request { Request::new(HttpMethod::Get, value) }
}

/// Convert a [`SmolPath`] to an [`http::Uri`], adding a leading slash.
#[cfg(feature = "http")]
pub fn smol_path_to_uri(
	path: SmolPath,
) -> Result<http::Uri, http::uri::InvalidUri> {
	http::Uri::try_from(path.with_leading_slash().as_str())
}

/// Build a [`SmolPath`] from [`RequestParts`].
pub fn smol_path_from_parts(parts: &RequestParts) -> SmolPath {
	SmolPath::from_segments(parts.path())
}

/// Convert a logical file path to a [`SmolPath`] suitable for URL routing.
/// - Extensions are removed
/// - `index` file stems are removed
/// - Leading/trailing `/` is stripped (by [`SmolPath`])
pub fn url_path_from_file_path(file_path: impl Into<SmolPath>) -> SmolPath {
	let path = file_path.into().with_extension("");
	if path.file_stem() == Some("index") {
		path.parent().unwrap_or_default()
	} else {
		path
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn smol_path_routing() {
		SmolPath::new("hello").to_string().xpect_eq("hello");

		for (value, expected) in [
			("hello", "hello"),
			("hello.rs", "hello"),
			("hello/index.rs", "hello"),
			("/hello/index/", "hello"),
			("/hello/index.rs/", "hello"),
			("/hello/index.rs", "hello"),
			("/index.rs", ""),
			("/index.rs/", ""),
			("/index/hi", "index/hi"),
			("/index/hi/", "index/hi"),
		] {
			url_path_from_file_path(value)
				.to_string()
				.xpect_eq(expected);
		}
	}

	#[beet_core::test]
	fn join() {
		SmolPath::new("foo")
			.join(&SmolPath::new("/"))
			.to_string()
			.xpect_eq("foo");
	}

	#[beet_core::test]
	fn from_segments() {
		let segments =
			vec!["api".to_string(), "users".to_string(), "123".to_string()];
		let path = SmolPath::from_segments(&segments);
		path.to_string().xpect_eq("api/users/123");
	}

	#[beet_core::test]
	fn from_segments_empty() {
		let path = SmolPath::from_segments(&Vec::<String>::new());
		path.to_string().xpect_eq("");
	}

	#[beet_core::test]
	fn segments() {
		let path = SmolPath::new("api/users/123");
		path.segments().xpect_eq(vec!["api", "users", "123"]);
	}

	#[beet_core::test]
	fn first_last_segment() {
		let path = SmolPath::new("api/users/123");
		path.first_segment().unwrap().xpect_eq("api");
		path.last_segment().unwrap().xpect_eq("123");

		let empty_path = SmolPath::default();
		empty_path.first_segment().xpect_none();
		empty_path.last_segment().xpect_none();
	}

	#[beet_core::test]
	fn from_request_parts() {
		let parts = RequestParts::get("/api/users/123");
		let path = smol_path_from_parts(&parts);
		path.to_string().xpect_eq("api/users/123");
	}
}
