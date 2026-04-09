use super::*;
use beet_core::prelude::*;
use std::path::Path;

/// Extract the relative path from request metadata.
impl FromRequestMeta<Self> for RelPath {
	fn from_request_meta(req: &RequestMeta) -> Result<Self, Response> {
		Self::new(req.path_string()).xok()
	}
}

impl From<RelPath> for Url {
	fn from(value: RelPath) -> Url {
		let path_str: &str = value.as_ref();
		Url::parse(path_str)
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

/// Convert a local file path to a [`RelPath`] suitable for URL routing.
/// - Extensions are removed
/// - `index` file stems are removed
/// - Leading `/` is stripped
pub fn url_path_from_file_path(file_path: impl AsRef<Path>) -> Result<RelPath> {
	let mut path = file_path.as_ref().to_path_buf();
	path.set_extension("");
	if path
		.file_stem()
		.and_then(|stem| stem.to_str())
		.is_some_and(|stem| stem == "index")
	{
		path.pop();
	}
	let mut raw_str = path.to_string_lossy().to_string();
	if raw_str.len() > 1 && raw_str.ends_with('/') {
		raw_str.pop();
	}
	Ok(RelPath::new(raw_str))
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
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
				.unwrap()
				.to_string()
				.xpect_eq(expected);
		}
	}

	#[test]
	fn join() {
		RelPath::new("foo")
			.join(&RelPath::new("/"))
			.to_string()
			.xpect_eq("foo");
	}

	#[test]
	fn from_segments() {
		let segments =
			vec!["api".to_string(), "users".to_string(), "123".to_string()];
		let path = RelPath::from_segments(&segments);
		path.to_string().xpect_eq("api/users/123");
	}

	#[test]
	fn from_segments_empty() {
		let path = RelPath::from_segments(&Vec::<String>::new());
		path.to_string().xpect_eq("");
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
	fn from_request_parts() {
		let parts = RequestParts::get("/api/users/123");
		let path = rel_path_from_parts(&parts);
		path.to_string().xpect_eq("api/users/123");
	}
}
