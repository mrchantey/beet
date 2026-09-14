use super::*;
use beet_core::prelude::*;

/// Extract the relative path from request metadata.
impl FromRequestMeta<Self> for RelPath {
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		Self::new(req.path_string()).xok()
	}
}

impl From<RelPath> for Request {
	fn from(value: RelPath) -> Request { Request::new(HttpMethod::Get, value) }
}

/// Convert a [`RelPath`] to an [`http::Uri`], adding a leading slash.
#[cfg(feature = "http")]
pub fn rel_path_to_uri(
	path: RelPath,
) -> Result<http::Uri, http::uri::InvalidUri> {
	http::Uri::try_from(path.with_leading_slash().as_str())
}

/// Build a [`RelPath`] from [`RequestParts`].
pub fn rel_path_from_parts(parts: &RequestParts) -> RelPath {
	RelPath::from_segments(parts.path())
}

/// Convert a logical file path to a [`RelPath`] suitable for URL routing.
/// - Extensions are removed
/// - `index` file stems are removed
/// - Leading/trailing `/` is stripped (by [`RelPath`])
pub fn url_path_from_file_path(file_path: impl Into<RelPath>) -> RelPath {
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
	fn rel_path_routing() {
		RelPath::new("hello").to_string().xpect_eq("hello");

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
		RelPath::new("foo")
			.join(&RelPath::new("/"))
			.to_string()
			.xpect_eq("foo");
	}

	#[beet_core::test]
	fn from_segments() {
		let segments =
			vec!["api".to_string(), "users".to_string(), "123".to_string()];
		let path = RelPath::from_segments(&segments);
		path.to_string().xpect_eq("api/users/123");
	}

	#[beet_core::test]
	fn from_segments_empty() {
		let path = RelPath::from_segments(&Vec::<String>::new());
		path.to_string().xpect_eq("");
	}

	#[beet_core::test]
	fn segments() {
		let path = RelPath::new("api/users/123");
		path.segments().xpect_eq(vec!["api", "users", "123"]);
	}

	#[beet_core::test]
	fn first_last_segment() {
		let path = RelPath::new("api/users/123");
		path.first_segment().unwrap().xpect_eq("api");
		path.last_segment().unwrap().xpect_eq("123");

		let empty_path = RelPath::default();
		empty_path.first_segment().xpect_none();
		empty_path.last_segment().xpect_none();
	}

	#[beet_core::test]
	fn from_request_parts() {
		let parts = RequestParts::get("/api/users/123");
		let path = rel_path_from_parts(&parts);
		path.to_string().xpect_eq("api/users/123");
	}
}
