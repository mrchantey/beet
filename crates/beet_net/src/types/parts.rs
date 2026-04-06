//! Generic request/response parts for routing.
//!
//! This module provides [`RequestParts`] and [`ResponseParts`] types
//! that abstract over different transport mechanisms (HTTP, CLI, etc.).
//!
//! These types allow the same routing infrastructure to handle:
//! - HTTP requests from a web server
//! - CLI commands parsed from arguments
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//! # use beet_net::prelude::*;
//! // From HTTP
//! let request = Request::get("/api/users?limit=10");
//! assert_eq!(request.path(), &["api", "users"]);
//! assert_eq!(request.get_param("limit"), Some("10"));
//! assert!(request.headers.get::<header::ContentType>().is_none());
//!
//! // From CLI: `myapp users list --limit 10`
//! let cli = CliArgs::parse("users list --limit 10");
//! let parts = RequestParts::from(cli);
//! assert_eq!(parts.path(), &["users", "list"]);
//! assert_eq!(parts.get_param("limit"), Some("10"));
//! ```

use super::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// The default HTTP version string.
const DEFAULT_HTTP_VERSION: &str = "1.1";

/// The default CLI version string.
const DEFAULT_CLI_VERSION: &str = "0.1.0";


/// Request-specific parts including HTTP method, URL, headers, and version.
///
/// # Example
///
/// ```
/// # use beet_net::prelude::*;
/// let parts = RequestParts::get("/api/users");
/// assert_eq!(parts.path(), &["api", "users"]);
/// assert_eq!(parts.method(), &HttpMethod::Get);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestParts {
	/// The HTTP method (GET, POST, etc.),
	///
	/// Note: body constraints may be enforced at the protocol level but they
	/// are not at the framework level, ie a GET request may have a body.
	method: HttpMethod,
	/// The URL components of the request.
	url: Url,
	/// HTTP headers or CLI environment variables.
	pub headers: HeaderMap,
	/// The HTTP version or CLI command version.
	version: Cow<'static, str>,
}

impl Default for RequestParts {
	fn default() -> Self {
		Self {
			method: HttpMethod::Get,
			url: Url::default(),
			headers: default(),
			version: Cow::Borrowed(DEFAULT_HTTP_VERSION),
		}
	}
}

impl RequestParts {
	/// Creates a new `RequestParts` with the given method and URL.
	///
	/// Accepts anything that converts [`Into<Url>`], including
	/// `&str`, `String`, and a pre-parsed [`Url`].
	pub fn new(method: HttpMethod, url: impl Into<Url>) -> Self {
		Self {
			method,
			url: url.into(),
			headers: HeaderMap::default(),
			version: Cow::Borrowed(DEFAULT_HTTP_VERSION),
		}
	}

	/// Creates a GET request for the given URL.
	pub fn get(url: impl Into<Url>) -> Self { Self::new(HttpMethod::Get, url) }

	/// Creates a POST request for the given URL.
	pub fn post(url: impl Into<Url>) -> Self {
		Self::new(HttpMethod::Post, url)
	}

	/// Creates a PUT request for the given URL.
	pub fn put(url: impl Into<Url>) -> Self { Self::new(HttpMethod::Put, url) }

	/// Creates a DELETE request for the given URL.
	pub fn delete(url: impl Into<Url>) -> Self {
		Self::new(HttpMethod::Delete, url)
	}

	/// Creates a PATCH request for the given URL.
	pub fn patch(url: impl Into<Url>) -> Self {
		Self::new(HttpMethod::Patch, url)
	}

	/// Returns the HTTP method.
	pub fn method(&self) -> &HttpMethod { &self.method }

	/// Sets the method.
	pub fn with_method(mut self, method: HttpMethod) -> Self {
		self.method = method;
		self
	}

	/// Returns the scheme.
	pub fn scheme(&self) -> &Scheme { self.url.scheme() }

	/// Returns the authority (host:port for HTTP).
	pub fn authority(&self) -> &str { self.url.authority().unwrap_or_default() }

	/// Returns the path segments.
	pub fn path(&self) -> &Vec<String> { self.url.path() }

	/// Returns the version string.
	pub fn version(&self) -> &str { &self.version }

	/// Returns all parameters.
	pub fn params(&self) -> &MultiMap<String, String> { self.url.params() }

	/// Returns a mutable reference to the parameters.
	pub fn params_mut(&mut self) -> &mut MultiMap<String, String> {
		self.url.params_mut()
	}

	/// Returns a mutable reference to the headers.
	pub fn headers_mut(&mut self) -> &mut HeaderMap { &mut self.headers }

	/// Adds a parameter and returns self for chaining.
	pub fn with_flag(mut self, key: impl Into<String>) -> Self {
		self.url.params_mut().insert_key(key.into());
		self
	}

	/// Adds a parameter and returns self for chaining.
	pub fn with_param(
		mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) -> Self {
		self.url.params_mut().insert(key.into(), value.into());
		self
	}

	/// Adds a flag parameter.
	pub fn insert_flag(&mut self, key: impl Into<String>) {
		self.url.params_mut().insert_key(key.into());
	}

	/// Adds a parameter.
	pub fn insert_param(
		&mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) {
		self.url.params_mut().insert(key.into(), value.into());
	}

	/// Returns all headers.
	pub fn headers(&self) -> &HeaderMap { &self.headers }

	/// Gets the first value for a parameter.
	pub fn get_param(&self, key: &str) -> Option<&str> {
		self.url.get_param(key)
	}

	/// Gets all values for a parameter.
	pub fn get_params(&self, key: &str) -> Option<&Vec<String>> {
		self.url.params().get_vec(key)
	}

	/// Checks if a parameter exists (useful for CLI flags).
	pub fn has_param(&self, key: &str) -> bool { self.url.has_param(key) }

	/// Check if this request indicates a body is present based on headers.
	pub fn has_body(&self) -> bool {
		self.headers
			.get::<header::ContentLength>()
			.and_then(|res| res.ok())
			.map(|len| len > 0)
			.unwrap_or(false)
			|| self
				.headers
				.get::<header::TransferEncoding>()
				.and_then(|res| res.ok())
				.map(|enc| {
					matches!(enc, header::TransferEncodingValue::Chunked)
				})
				.unwrap_or(false)
	}

	/// Returns the path as a joined string with leading slash.
	pub fn path_string(&self) -> String { self.url.path_string() }

	/// Returns the query string built from params.
	/// This is the canonical way to get the query string.
	pub fn query_string(&self) -> String { self.url.query_string() }

	/// Returns the full URI string, lazily constructed from scheme, authority, path, and params.
	pub fn uri(&self) -> String { self.url.to_string() }

	/// Returns the first path segment, if any.
	pub fn first_segment(&self) -> Option<&str> { self.url.first_segment() }

	/// Returns the last path segment, if any.
	pub fn last_segment(&self) -> Option<&str> { self.url.last_segment() }

	/// Returns path segments starting from the given index.
	pub fn path_from(&self, index: usize) -> &[String] {
		self.url.path_from(index)
	}

	/// Returns a reference to the inner [`Url`].
	pub fn url(&self) -> &Url { &self.url }

	/// Returns a mutable reference to the inner [`Url`].
	pub fn url_mut(&mut self) -> &mut Url { &mut self.url }
}


/// Response-specific parts including HTTP status code, headers, and version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseParts {
	/// The HTTP status code of the response.
	pub status: StatusCode,
	/// HTTP headers.
	pub headers: HeaderMap,
	/// The HTTP version.
	pub version: Cow<'static, str>,
}

impl Default for ResponseParts {
	fn default() -> Self {
		Self {
			status: StatusCode::OK,
			headers: default(),
			version: Cow::Borrowed(DEFAULT_HTTP_VERSION),
		}
	}
}

impl ResponseParts {
	/// Creates a new `ResponseParts` with the given status.
	pub fn new(status: StatusCode) -> Self {
		Self {
			status,
			headers: default(),
			version: Cow::Borrowed(DEFAULT_HTTP_VERSION),
		}
	}

	/// Creates an OK (200) response.
	pub fn ok() -> Self { Self::new(StatusCode::OK) }

	/// Creates a Not Found (404) response.
	pub fn not_found() -> Self { Self::new(StatusCode::NOT_FOUND) }

	/// Creates an Internal Server Error (500) response.
	pub fn internal_error() -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR)
	}

	/// Creates a Bad Request (400) response.
	pub fn bad_request() -> Self { Self::new(StatusCode::BAD_REQUEST) }

	/// Returns the status code.
	pub fn status(&self) -> StatusCode { self.status }

	/// Use exit code conventions to map a status to an exit code.
	pub fn status_to_exit_code(&self) -> Result<(), std::num::NonZeroU8> {
		self.status().to_exit_code()
	}

	/// Returns the version string.
	pub fn version(&self) -> &str { &self.version }

	/// Returns all headers.
	pub fn headers(&self) -> &HeaderMap { &self.headers }

	/// Returns a mutable reference to the headers.
	pub fn headers_mut(&mut self) -> &mut HeaderMap { &mut self.headers }

	/// Sets the status code.
	pub fn with_status(mut self, status: StatusCode) -> Self {
		self.status = status;
		self
	}
}

// ============================================================================
// Conversion: http::request::Parts -> RequestParts
// ============================================================================

#[cfg(feature = "http")]
impl From<http::request::Parts> for RequestParts {
	fn from(http_parts: http::request::Parts) -> Self {
		let uri = &http_parts.uri;

		let scheme = Scheme::from(uri.scheme());
		let authority = uri.authority().map(|auth| auth.to_string());
		let path = split_path(uri.path());
		let params = uri.query().map(parse_query_string).unwrap_or_default();
		let headers = http_header_map_to_header_map(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);
		let method = HttpMethod::from(http_parts.method);

		let url = Url::new(scheme, authority, path, params, None);

		RequestParts {
			method,
			url,
			headers,
			version: Cow::Owned(version),
		}
	}
}

#[cfg(feature = "http")]
impl From<&http::request::Parts> for RequestParts {
	fn from(http_parts: &http::request::Parts) -> Self {
		let uri = &http_parts.uri;

		let scheme = Scheme::from(uri.scheme());
		let authority = uri.authority().map(|auth| auth.to_string());
		let path = split_path(uri.path());
		let params = uri.query().map(parse_query_string).unwrap_or_default();
		let headers = http_header_map_to_header_map(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);
		let method = HttpMethod::from(&http_parts.method);

		let url = Url::new(scheme, authority, path, params, None);

		RequestParts {
			method,
			url,
			headers,
			version: Cow::Owned(version),
		}
	}
}

// ============================================================================
// Conversion: http::response::Parts -> ResponseParts
// ============================================================================

#[cfg(feature = "http")]
impl From<http::response::Parts> for ResponseParts {
	fn from(http_parts: http::response::Parts) -> Self {
		let headers = http_header_map_to_header_map(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);

		ResponseParts {
			status: StatusCode::from(http_parts.status),
			headers,
			version: Cow::Owned(version),
		}
	}
}


#[cfg(feature = "http")]
impl From<&http::response::Parts> for ResponseParts {
	fn from(http_parts: &http::response::Parts) -> Self {
		let headers = http_header_map_to_header_map(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);

		ResponseParts {
			status: StatusCode::from(http_parts.status),
			headers,
			version: Cow::Owned(version),
		}
	}
}

// ============================================================================
// Conversion: CliArgs -> RequestParts
// ============================================================================

impl From<CliArgs> for RequestParts {
	fn from(cli: CliArgs) -> Self {
		let url = Url::new(Scheme::None, None, cli.path, cli.params, None);
		RequestParts {
			method: HttpMethod::Get,
			url,
			headers: HeaderMap::default(),
			version: Cow::Owned(
				env_ext::var("CARGO_PKG_VERSION")
					.unwrap_or_else(|_| DEFAULT_CLI_VERSION.to_string()),
			),
		}
	}
}

// ============================================================================
// Conversion: RequestParts/ResponseParts -> http types
// ============================================================================

#[cfg(feature = "http")]
impl TryFrom<RequestParts> for http::request::Parts {
	type Error = http::Error;

	fn try_from(parts: RequestParts) -> Result<Self, Self::Error> {
		let method: http::Method = parts.method.into();

		// Build URI from path and params
		let uri_str = parts.uri();

		let mut builder = http::Request::builder()
			.method(method)
			.uri(&uri_str)
			.version(http_ext::parse_version(&parts.version));

		// Add headers
		if let Ok(header_map) = header_map_to_http(&parts.headers) {
			for (key, value) in header_map.iter() {
				builder = builder.header(key, value);
			}
		}

		let (http_parts, _) = builder.body(())?.into_parts();
		Ok(http_parts)
	}
}

#[cfg(feature = "http")]
impl TryFrom<ResponseParts> for http::response::Parts {
	type Error = http::Error;

	fn try_from(parts: ResponseParts) -> Result<Self, Self::Error> {
		let mut builder = http::Response::builder()
			.status(parts.status)
			.version(http_ext::parse_version(&parts.version));

		// Add headers
		if let Ok(header_map) = header_map_to_http(&parts.headers) {
			for (key, value) in header_map.iter() {
				builder = builder.header(key, value);
			}
		}

		let (http_parts, _) = builder.body(())?.into_parts();
		Ok(http_parts)
	}
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn request_parts_default() {
		let parts = RequestParts::default();
		parts.path().xpect_empty();
		parts.uri().xpect_eq("/");
		parts.scheme().clone().xpect_eq(Scheme::None);
		parts.version().xpect_eq("1.1");
	}

	#[test]
	fn request_parts_with_headers_and_params() {
		let mut parts = RequestParts::new(
			HttpMethod::Get,
			"http://example.com/api/users/123?limit=10",
		);
		parts.headers.set_content_type(MediaType::Json);

		parts.scheme().clone().xpect_eq(Scheme::Http);
		parts.authority().xpect_eq("example.com");
		parts.path().xpect_eq(vec![
			"api".to_string(),
			"users".to_string(),
			"123".to_string(),
		]);
		parts.get_param("limit").unwrap().xpect_eq("10");
		parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn request_parts_with_params() {
		let parts = RequestParts::new(HttpMethod::Post, "/api/users?page=1");

		(*parts.method()).xpect_eq(HttpMethod::Post);
		parts
			.path()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		parts.get_param("page").unwrap().xpect_eq("1");
	}

	#[test]
	fn response_parts_with_headers() {
		let mut parts = ResponseParts::new(StatusCode::OK);
		parts.headers.set_content_type(MediaType::Html);

		parts.status().xpect_eq(StatusCode::OK);
		parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Html);
	}

	#[test]
	fn request_parts_new() {
		let parts = RequestParts::get("/api/users");
		(*parts.method()).xpect_eq(HttpMethod::Get);
		parts
			.path()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
	}

	#[test]
	fn request_parts_post() {
		let parts = RequestParts::post("/api/users");
		(*parts.method()).xpect_eq(HttpMethod::Post);
	}

	#[test]
	fn response_parts_default() {
		let parts = ResponseParts::default();
		parts.status().xpect_eq(StatusCode::OK);
	}

	#[test]
	fn response_parts_not_found() {
		let parts = ResponseParts::not_found();
		parts.status().xpect_eq(StatusCode::NOT_FOUND);
	}

	#[test]
	#[cfg(feature = "http")]
	fn from_http_request_parts() {
		let http_parts = http::Request::builder()
			.method(http::Method::POST)
			.uri("https://example.com/api/users?limit=10&offset=20")
			.header("content-type", "application/json")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		let parts = RequestParts::from(http_parts);

		(*parts.method()).xpect_eq(HttpMethod::Post);
		parts.scheme().clone().xpect_eq(Scheme::Https);
		parts.authority().xpect_eq("example.com");
		parts
			.path()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		parts.get_param("limit").unwrap().xpect_eq("10");
		parts.get_param("offset").unwrap().xpect_eq("20");
		parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
	}

	#[test]
	#[cfg(feature = "http")]
	fn from_http_response_parts() {
		let http_parts = http::Response::builder()
			.status(http::StatusCode::CREATED)
			.header("content-type", "application/json")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		let parts = ResponseParts::from(http_parts);

		parts.status().xpect_eq(StatusCode::CREATED);
		parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn from_cli_args() {
		let cli = CliArgs::parse("users list --limit 10 --verbose");
		let parts = RequestParts::from(cli);

		parts.scheme().clone().xpect_eq(Scheme::None);
		(*parts.method()).xpect_eq(HttpMethod::Get);
		parts
			.path()
			.xpect_eq(vec!["users".to_string(), "list".to_string()]);
		parts.get_param("limit").unwrap().xpect_eq("10");
		parts.has_param("verbose").xpect_true();
	}

	#[test]
	fn from_cli_args_flags_only() {
		let cli = CliArgs::parse("--verbose --debug");
		let parts = RequestParts::from(cli);

		parts.path().xpect_empty();
		parts.has_param("verbose").xpect_true();
		parts.has_param("debug").xpect_true();
	}

	#[test]
	fn path_string() {
		let parts = RequestParts::get("/api/users/123");
		parts.path_string().xpect_eq("/api/users/123");

		let empty_parts = RequestParts::default();
		empty_parts.path_string().xpect_eq("/");
	}

	#[test]
	fn query_string() {
		let parts = RequestParts::get("/?limit=10&offset=20");
		let query = parts.query_string();
		// Order may vary, so check both params are present
		(&query).xpect_contains("limit=10");
		(&query).xpect_contains("offset=20");
	}

	#[test]
	fn uri_construction() {
		let parts = RequestParts::get("/api/users?page=1");
		let uri = parts.uri();
		(&uri).xpect_starts_with("/api/users?");
		(&uri).xpect_contains("page=1");
	}

	#[test]
	fn path_segments() {
		let parts = RequestParts::get("/api/users/123");

		parts.first_segment().unwrap().xpect_eq("api");
		parts.last_segment().unwrap().xpect_eq("123");
		parts.path_from(1).xpect_eq(["users", "123"]);
		parts.path_from(10).len().xpect_eq(0);
	}

	#[test]
	#[cfg(feature = "http")]
	fn request_parts_to_http() {
		let mut parts = RequestParts::post("/api/users?limit=10");
		parts.headers.set_content_type(MediaType::Json);

		let http_parts: http::request::Parts = parts.try_into().unwrap();

		http_parts.method.xpect_eq(http::Method::POST);
		http_parts.uri.path().xpect_eq("/api/users");
	}

	#[test]
	#[cfg(feature = "http")]
	fn response_parts_to_http() {
		let mut parts = ResponseParts::new(StatusCode::CREATED);
		parts.headers.set_content_type(MediaType::Json);

		let http_parts: http::response::Parts = parts.try_into().unwrap();

		http_parts.status.xpect_eq(http::StatusCode::CREATED);
	}

	#[test]
	fn request_parts_access() {
		let parts = RequestParts::get("/api/users");
		// Access RequestParts methods directly
		parts.path().len().xpect_eq(2);
		parts.path_string().xpect_eq("/api/users");
	}

	#[test]
	fn response_parts_headers() {
		let mut parts = ResponseParts::ok();
		parts.headers.set_raw("x-custom", "value");
		parts
			.headers
			.first_raw("x-custom")
			.unwrap()
			.xpect_eq("value");
	}

	#[test]
	fn has_body_detection() {
		let mut parts = RequestParts::default();
		parts.has_body().xpect_false();

		parts.headers.set::<header::ContentLength>(5u64);
		parts.has_body().xpect_true();

		let mut parts2 = RequestParts::default();
		parts2.headers.set::<header::TransferEncoding>(
			header::TransferEncodingValue::Chunked,
		);
		parts2.has_body().xpect_true();
	}

	#[test]
	fn version_is_cow_static() {
		// Default version uses borrowed static string
		let parts = RequestParts::default();
		parts.version().xpect_eq("1.1");

		let response = ResponseParts::default();
		response.version().xpect_eq("1.1");
	}
}
