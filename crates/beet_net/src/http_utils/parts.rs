//! Generic request/response parts for routing.
//!
//! This module provides [`Parts`], [`RequestParts`], and [`ResponseParts`] types
//! that abstract over different transport mechanisms (HTTP, CLI, REPL, etc.).
//!
//! These types allow the same routing infrastructure to handle:
//! - HTTP requests from a web server
//! - CLI commands parsed from arguments
//! - REPL commands in interactive sessions
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use beet_core::prelude::*;
//! // From HTTP
//! let request = Request::get("/api/users?limit=10");
//! assert_eq!(request.path(), &["api", "users"]);
//! assert_eq!(request.get_param("limit"), Some(&"10".to_string()));
//!
//! // From CLI: `myapp users list --limit 10`
//! let cli = CliArgs::parse("users list --limit 10");
//! let parts = RequestParts::from(cli);
//! assert_eq!(parts.path(), &["users", "list"]);
//! assert_eq!(parts.get_param("limit"), Some(&"10".to_string()));
//! ```

use crate::prelude::*;
use beet_core::prelude::*;


/// The transport scheme used for a request or response.
///
/// This indicates how the request was received or how the response
/// should be formatted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Scheme {
	/// No scheme specified
	#[default]
	None,
	/// The request was made over HTTP
	Http,
	/// The request was made over HTTPS
	Https,
	/// The request was made as CLI arguments
	Cli,
	/// The request was made as a REPL command
	Repl,
	/// Some other scheme not listed here
	Other(String),
}

impl Scheme {
	/// Parse a scheme from a string
	pub fn from_str(scheme: &str) -> Self {
		match scheme.to_ascii_lowercase().as_str() {
			"http" => Self::Http,
			"https" => Self::Https,
			"cli" => Self::Cli,
			"repl" => Self::Repl,
			"" => Self::None,
			other => Self::Other(other.to_string()),
		}
	}

	/// Convert the scheme to a string representation
	pub fn as_str(&self) -> &str {
		match self {
			Self::None => "",
			Self::Http => "http",
			Self::Https => "https",
			Self::Cli => "cli",
			Self::Repl => "repl",
			Self::Other(scheme) => scheme.as_str(),
		}
	}

	/// Whether this is an HTTP-based scheme
	pub fn is_http(&self) -> bool { matches!(self, Self::Http | Self::Https) }

	/// Whether this is a CLI-based scheme
	pub fn is_cli(&self) -> bool { matches!(self, Self::Cli | Self::Repl) }
}

impl std::fmt::Display for Scheme {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{}", self.as_str())
	}
}

impl From<&http::uri::Scheme> for Scheme {
	fn from(scheme: &http::uri::Scheme) -> Self {
		Self::from_str(scheme.as_str())
	}
}

impl From<Option<&http::uri::Scheme>> for Scheme {
	fn from(scheme: Option<&http::uri::Scheme>) -> Self {
		scheme.map(Self::from).unwrap_or(Self::None)
	}
}

/// Common parts shared between requests and responses.
///
/// This type abstracts the commonalities between different transport mechanisms:
/// - For HTTP: scheme, host, path segments, query params, headers
/// - For CLI: command path segments, flags/options, environment variables
///
/// # Path Representation
///
/// The path is stored as a vector of segments:
/// - HTTP `/api/users/123` -> `["api", "users", "123"]`
/// - CLI `users list --verbose` -> `["users", "list"]`
///
/// # Parameters
///
/// Query parameters (HTTP) and flags/options (CLI) are unified:
/// - HTTP `?limit=10&offset=20` -> `{"limit": ["10"], "offset": ["20"]}`
/// - CLI `--limit 10 --offset 20` -> `{"limit": ["10"], "offset": ["20"]}`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parts {
	/// The scheme of the request (http, https, cli, etc.)
	scheme: Scheme,
	/// The authority (host:port) for HTTP, or application name for CLI
	authority: String,
	/// The path segments (split by `/` for HTTP, positional args for CLI)
	path: Vec<String>,
	/// Query parameters (HTTP) or flags/options (CLI)
	///
	/// For CLI flags:
	/// - Short and long versions are stored separately: `--foo` and `-f`
	/// - Flags without values have empty vectors
	params: MultiMap<String, String>,
	/// HTTP headers or CLI environment variables
	headers: MultiMap<String, String>,
	/// The HTTP version or CLI command version
	version: String,
}

impl Default for Parts {
	fn default() -> Self {
		Self {
			scheme: Scheme::None,
			authority: String::new(),
			path: Vec::new(),
			params: default(),
			headers: default(),
			version: http_ext::DEFAULT_HTTP_VERSION.to_string(),
		}
	}
}

impl Parts {
	/// Creates a new empty `Parts` with default values
	pub fn new() -> Self { Self::default() }

	/// Returns the scheme
	pub fn scheme(&self) -> &Scheme { &self.scheme }

	/// Returns the authority (host:port for HTTP, app name for CLI)
	pub fn authority(&self) -> &str { &self.authority }

	/// Returns the path segments
	pub fn path(&self) -> &Vec<String> { &self.path }

	/// Returns the version string
	pub fn version(&self) -> &str { &self.version }

	/// Returns all parameters
	pub fn params(&self) -> &MultiMap<String, String> { &self.params }

	/// Returns a mutable reference to the parameters
	pub fn params_mut(&mut self) -> &mut MultiMap<String, String> {
		&mut self.params
	}

	/// Returns a mutable reference to the headers
	pub fn headers_mut(&mut self) -> &mut MultiMap<String, String> {
		&mut self.headers
	}

	/// Adds a parameter
	pub fn insert_param(
		&mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) {
		self.params.insert(key.into(), value.into());
	}

	/// Adds a header
	pub fn insert_header(
		&mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) {
		self.headers.insert(key.into(), value.into());
	}

	/// Returns all headers
	pub fn headers(&self) -> &MultiMap<String, String> { &self.headers }

	/// Gets the first value for a parameter
	pub fn get_param(&self, key: &str) -> Option<&String> {
		self.params.get_vec(key).and_then(|vals| vals.first())
	}

	/// Gets all values for a parameter
	pub fn get_params(&self, key: &str) -> Option<&Vec<String>> {
		self.params.get_vec(key)
	}

	/// Gets the first value for a header
	pub fn get_header(&self, key: &str) -> Option<&String> {
		self.headers.get_vec(key).and_then(|vals| vals.first())
	}

	/// Gets all values for a header
	pub fn get_headers(&self, key: &str) -> Option<&Vec<String>> {
		self.headers.get_vec(key)
	}

	/// Checks if a parameter exists (useful for CLI flags)
	pub fn has_param(&self, key: &str) -> bool { self.params.contains_key(key) }

	/// Checks if a header exists
	pub fn has_header(&self, key: &str) -> bool {
		self.headers.contains_key(key)
	}

	/// Check if this request indicates a body is present based on headers.
	pub fn has_body(&self) -> bool {
		self.get_header("content-length")
			.and_then(|val| val.parse::<usize>().ok())
			.map(|len| len > 0)
			.unwrap_or(false)
			|| self
				.get_header("transfer-encoding")
				.map(|val| val.contains("chunked"))
				.unwrap_or(false)
	}

	/// Returns the path as a joined string with leading slash
	pub fn path_string(&self) -> String {
		if self.path.is_empty() {
			"/".to_string()
		} else {
			format!("/{}", self.path.join("/"))
		}
	}

	/// Returns the query string built from params.
	/// This is the canonical way to get the query string.
	pub fn query_string(&self) -> String { build_query_string(&self.params) }

	/// Returns the full URI string, lazily constructed from scheme, authority, path, and params.
	pub fn uri(&self) -> String {
		let path = self.path_string();
		let query = self.query_string();

		// Build base with scheme and authority if present
		let base = match (&self.scheme, self.authority.is_empty()) {
			(Scheme::None, _) | (_, true) => path,
			(scheme, false) => {
				format!("{}://{}{}", scheme.as_str(), self.authority, path)
			}
		};

		if query.is_empty() {
			base
		} else {
			format!("{}?{}", base, query)
		}
	}

	/// Returns the first path segment, if any
	pub fn first_segment(&self) -> Option<&str> {
		self.path.first().map(|segment| segment.as_str())
	}

	/// Returns the last path segment, if any
	pub fn last_segment(&self) -> Option<&str> {
		self.path.last().map(|segment| segment.as_str())
	}

	/// Returns path segments starting from the given index
	pub fn path_from(&self, index: usize) -> &[String] {
		if index >= self.path.len() {
			&[]
		} else {
			&self.path[index..]
		}
	}
}

/// Builder for constructing [`Parts`], [`RequestParts`], or [`ResponseParts`].
#[derive(Debug, Clone, Default)]
pub struct PartsBuilder {
	scheme: Scheme,
	authority: String,
	path: Vec<String>,
	params: MultiMap<String, String>,
	headers: MultiMap<String, String>,
	version: Option<String>,
}

impl PartsBuilder {
	/// Creates a new builder with default values
	pub fn new() -> Self { Self::default() }

	/// Sets the scheme
	pub fn scheme(mut self, scheme: Scheme) -> Self {
		self.scheme = scheme;
		self
	}

	/// Sets the authority
	pub fn authority(mut self, authority: impl Into<String>) -> Self {
		self.authority = authority.into();
		self
	}

	/// Sets the path from segments
	pub fn path(mut self, path: Vec<String>) -> Self {
		self.path = path;
		self
	}

	/// Sets the path from a string, splitting by `/`
	pub fn path_str(mut self, path: &str) -> Self {
		self.path = split_path(path);
		self
	}

	/// Adds a parameter
	pub fn param(
		mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) -> Self {
		self.params.insert(key.into(), value.into());
		self
	}

	/// Adds a flag parameter (parameter with no value)
	pub fn flag(mut self, key: impl Into<String>) -> Self {
		let key = key.into();
		if !self.params.contains_key(&key) {
			self.params.insert(key, String::new());
		}
		self
	}

	/// Adds a header
	pub fn header(
		mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) -> Self {
		self.headers.insert(key.into(), value.into());
		self
	}

	/// Sets the version
	pub fn version(mut self, version: impl Into<String>) -> Self {
		self.version = Some(version.into());
		self
	}

	/// Builds the [`Parts`]
	pub fn build(self) -> Parts {
		Parts {
			scheme: self.scheme,
			authority: self.authority,
			path: self.path,
			params: self.params,
			headers: self.headers,
			version: self
				.version
				.unwrap_or_else(|| http_ext::DEFAULT_HTTP_VERSION.to_string()),
		}
	}

	/// Builds [`RequestParts`] with the given method
	pub fn build_request_parts(self, method: HttpMethod) -> RequestParts {
		RequestParts {
			method,
			parts: self.build(),
		}
	}

	/// Builds [`ResponseParts`] with the given status
	pub fn build_response_parts(self, status: StatusCode) -> ResponseParts {
		ResponseParts {
			status,
			parts: self.build(),
		}
	}
}

/// Request-specific parts including HTTP method.
///
/// This wraps [`Parts`] with request-specific data like the HTTP method.
/// For CLI commands, the method defaults to [`HttpMethod::Get`].
///
/// # Deref
///
/// `RequestParts` implements `Deref<Target = Parts>`, so all methods
/// on [`Parts`] are available directly:
///
/// ```
/// # use beet_net::prelude::*;
/// let parts = RequestParts::get("/api/users");
/// assert_eq!(parts.path(), &["api", "users"]); // Deref to Parts
/// assert_eq!(parts.method(), &HttpMethod::Get);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestParts {
	method: HttpMethod,
	parts: Parts,
}

impl Default for RequestParts {
	fn default() -> Self {
		Self {
			method: HttpMethod::Get,
			parts: Parts::default(),
		}
	}
}

impl RequestParts {
	/// Creates a new `RequestParts` with the given method and path or URI.
	///
	/// The input can be:
	/// - A path like `/api/users`
	/// - A full URI like `https://example.com/api/users`
	/// - A CLI-style path like `users list`
	pub fn new(method: HttpMethod, path: impl AsRef<str>) -> Self {
		let path_str = path.as_ref();

		// Check if this is a full URI with scheme (http://, https://, etc.)
		if path_str.contains("://") {
			// Parse as full URI
			if let Ok(uri) = path_str.parse::<http::Uri>() {
				let scheme = Scheme::from(uri.scheme());
				let authority = uri
					.authority()
					.map(|auth| auth.to_string())
					.unwrap_or_default();
				let path_segments = split_path(uri.path());
				let params =
					uri.query().map(parse_query_string).unwrap_or_default();

				return Self {
					method,
					parts: Parts {
						scheme,
						authority,
						path: path_segments,
						params,
						headers: MultiMap::<String, String>::default(),
						version: http_ext::DEFAULT_HTTP_VERSION.to_string(),
					},
				};
			}
		}

		// Otherwise treat as path (may include query string)
		let (path_only, query_str) = split_path_and_query(path_str);
		let path_segments = split_path(path_only);
		let params = query_str.map(parse_query_string).unwrap_or_default();

		Self {
			method,
			parts: Parts {
				scheme: Scheme::None,
				authority: String::new(),
				path: path_segments,
				params,
				headers: MultiMap::<String, String>::default(),
				version: http_ext::DEFAULT_HTTP_VERSION.to_string(),
			},
		}
	}

	/// Creates a GET request for the given path
	pub fn get(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Get, path)
	}

	/// Creates a POST request for the given path
	pub fn post(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Post, path)
	}

	/// Creates a PUT request for the given path
	pub fn put(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Put, path)
	}

	/// Creates a DELETE request for the given path
	pub fn delete(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Delete, path)
	}

	/// Creates a PATCH request for the given path
	pub fn patch(path: impl AsRef<str>) -> Self {
		Self::new(HttpMethod::Patch, path)
	}

	/// Returns the HTTP method
	pub fn method(&self) -> &HttpMethod { &self.method }

	/// Returns a mutable reference to the inner parts
	pub fn parts_mut(&mut self) -> &mut Parts { &mut self.parts }

	/// Consumes self and returns the inner parts
	pub fn into_parts(self) -> Parts { self.parts }

	/// Sets the method
	pub fn with_method(mut self, method: HttpMethod) -> Self {
		self.method = method;
		self
	}
}

impl std::ops::Deref for RequestParts {
	type Target = Parts;
	fn deref(&self) -> &Self::Target { &self.parts }
}

impl std::ops::DerefMut for RequestParts {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.parts }
}

/// Response-specific parts including HTTP status code.
///
/// This wraps [`Parts`] with response-specific data like the status code.
///
/// # Deref
///
/// `ResponseParts` implements `Deref<Target = Parts>`, so all methods
/// on [`Parts`] are available directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseParts {
	status: StatusCode,
	parts: Parts,
}

impl Default for ResponseParts {
	fn default() -> Self {
		Self {
			status: StatusCode::OK,
			parts: Parts::default(),
		}
	}
}

impl ResponseParts {
	/// Creates a new `ResponseParts` with the given status
	pub fn new(status: StatusCode) -> Self {
		Self {
			status,
			parts: Parts::default(),
		}
	}

	/// Creates an OK (200) response
	pub fn ok() -> Self { Self::new(StatusCode::OK) }

	/// Creates a Not Found (404) response
	pub fn not_found() -> Self { Self::new(StatusCode::NOT_FOUND) }

	/// Creates an Internal Server Error (500) response
	pub fn internal_error() -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR)
	}

	/// Creates a Bad Request (400) response
	pub fn bad_request() -> Self { Self::new(StatusCode::BAD_REQUEST) }

	/// Returns the status code
	pub fn status(&self) -> StatusCode { self.status }

	/// Returns a mutable reference to the inner parts
	pub fn parts_mut(&mut self) -> &mut Parts { &mut self.parts }

	/// Consumes self and returns the inner parts
	pub fn into_parts(self) -> Parts { self.parts }

	/// Sets the status code
	pub fn with_status(mut self, status: StatusCode) -> Self {
		self.status = status;
		self
	}
}

impl std::ops::Deref for ResponseParts {
	type Target = Parts;
	fn deref(&self) -> &Self::Target { &self.parts }
}

impl std::ops::DerefMut for ResponseParts {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.parts }
}

// ============================================================================
// Private parsing helpers
// ============================================================================

/// Split a URI string into path and optional query string.
fn split_path_and_query(uri: &str) -> (&str, Option<&str>) {
	uri.split_once('?')
		.map(|(path, query)| (path, Some(query)))
		.unwrap_or((uri, None))
}

/// Convert an [`http::HeaderMap`] to a [`MultiMap<String, String>`],
/// with all keys converted to lower kebab-case
fn header_map_to_multimap(map: &http::HeaderMap) -> MultiMap<String, String> {
	use heck::ToKebabCase;
	let mut multi_map = MultiMap::default();
	for (key, value) in map.iter() {
		let key = key.to_string().to_kebab_case();
		// Header values can technically contain opaque bytes
		// but this is considered bad practice; we use a placeholder
		let value = value.to_str().unwrap_or("<opaque-bytes>").to_string();
		multi_map.insert(key, value);
	}
	multi_map
}

/// Parse query string into a MultiMap
fn parse_query_string(query: &str) -> MultiMap<String, String> {
	let mut params = MultiMap::default();
	for pair in query.split('&') {
		if pair.is_empty() {
			continue;
		}
		let (key, value) = match pair.split_once('=') {
			Some((key, value)) => (key.to_string(), value.to_string()),
			None => (pair.to_string(), String::new()),
		};
		params.insert(key, value);
	}
	params
}

/// Split a path string into segments, filtering empty segments
fn split_path(path: &str) -> Vec<String> {
	path.split('/')
		.filter(|segment| !segment.is_empty())
		.map(|segment| segment.to_string())
		.collect()
}

/// Convert a MultiMap back to http::HeaderMap
fn multimap_to_header_map(
	multimap: &MultiMap<String, String>,
) -> Result<http::HeaderMap, http::header::InvalidHeaderValue> {
	use std::str::FromStr;
	let mut headers = http::HeaderMap::new();
	for (key, values) in multimap.iter_all() {
		let header_name = http::header::HeaderName::from_str(key)
			.unwrap_or_else(|_| {
				http::header::HeaderName::from_static("x-invalid")
			});
		for value in values {
			headers.append(
				header_name.clone(),
				http::header::HeaderValue::from_str(value)?,
			);
		}
	}
	Ok(headers)
}

/// Build query string from MultiMap
fn build_query_string(params: &MultiMap<String, String>) -> String {
	let mut parts = Vec::new();
	for (key, values) in params.iter_all() {
		for value in values {
			if value.is_empty() {
				parts.push(key.clone());
			} else {
				parts.push(format!("{}={}", key, value));
			}
		}
	}
	parts.join("&")
}

// ============================================================================
// Conversion: http::request::Parts -> RequestParts
// ============================================================================

impl From<http::request::Parts> for RequestParts {
	fn from(http_parts: http::request::Parts) -> Self {
		let uri = &http_parts.uri;

		let scheme = Scheme::from(uri.scheme());
		let authority = uri
			.authority()
			.map(|auth| auth.to_string())
			.unwrap_or_default();
		let path = split_path(uri.path());
		let params = uri.query().map(parse_query_string).unwrap_or_default();
		let headers = header_map_to_multimap(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);
		let method = HttpMethod::from(http_parts.method);

		RequestParts {
			method,
			parts: Parts {
				scheme,
				authority,
				path,
				params,
				headers,
				version,
			},
		}
	}
}

impl From<&http::request::Parts> for RequestParts {
	fn from(http_parts: &http::request::Parts) -> Self {
		let uri = &http_parts.uri;

		let scheme = Scheme::from(uri.scheme());
		let authority = uri
			.authority()
			.map(|auth| auth.to_string())
			.unwrap_or_default();
		let path = split_path(uri.path());
		let params = uri.query().map(parse_query_string).unwrap_or_default();
		let headers = header_map_to_multimap(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);
		let method = HttpMethod::from(&http_parts.method);

		RequestParts {
			method,
			parts: Parts {
				scheme,
				authority,
				path,
				params,
				headers,
				version,
			},
		}
	}
}

// ============================================================================
// Conversion: http::response::Parts -> ResponseParts
// ============================================================================

impl From<http::response::Parts> for ResponseParts {
	fn from(http_parts: http::response::Parts) -> Self {
		let headers = header_map_to_multimap(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);

		ResponseParts {
			status: http_parts.status,
			parts: Parts {
				scheme: Scheme::None,
				authority: String::new(),
				path: Vec::new(),
				params: MultiMap::default(),
				headers,
				version,
			},
		}
	}
}

impl From<&http::response::Parts> for ResponseParts {
	fn from(http_parts: &http::response::Parts) -> Self {
		let headers = header_map_to_multimap(&http_parts.headers);
		let version = http_ext::version_to_string(http_parts.version);

		ResponseParts {
			status: http_parts.status,
			parts: Parts {
				scheme: Scheme::None,
				authority: String::new(),
				path: Vec::new(),
				params: MultiMap::default(),
				headers,
				version,
			},
		}
	}
}

// ============================================================================
// Conversion: CliArgs -> RequestParts
// ============================================================================

impl From<CliArgs> for RequestParts {
	fn from(cli: CliArgs) -> Self {
		// For CLI, path segments are the positional arguments verbatim
		let path = cli.path.clone();

		// Convert query HashMap to MultiMap before consuming cli
		// We need to capture flags (keys with empty value vectors) as well
		let mut params = MultiMap::default();
		for (key, values) in &cli.query {
			if values.is_empty() {
				// Flag without value - insert with empty string to mark presence
				params.insert(key.clone(), String::new());
			} else {
				for value in values {
					params.insert(key.clone(), value.clone());
				}
			}
		}

		RequestParts {
			method: HttpMethod::Get, // CLI defaults to GET-like semantics
			parts: Parts {
				scheme: Scheme::Cli,
				authority: env_ext::var("CARGO_PKG_NAME").unwrap_or_default(),
				path,
				params,
				headers: MultiMap::default(),
				version: env_ext::var("CARGO_PKG_VERSION").unwrap_or_else(
					|_| http_ext::DEFAULT_CLI_VERSION.to_string(),
				),
			},
		}
	}
}

impl From<&CliArgs> for RequestParts {
	fn from(cli: &CliArgs) -> Self {
		let path = cli.path.clone();

		let mut params = MultiMap::default();
		for (key, values) in &cli.query {
			if values.is_empty() {
				params.insert(key.clone(), String::new());
			} else {
				for value in values {
					params.insert(key.clone(), value.clone());
				}
			}
		}

		RequestParts {
			method: HttpMethod::Get,
			parts: Parts {
				scheme: Scheme::Cli,
				authority: std::env::var("CARGO_PKG_NAME").unwrap_or_default(),
				path,
				params,
				headers: MultiMap::default(),
				version: http_ext::DEFAULT_CLI_VERSION.to_string(),
			},
		}
	}
}

// ============================================================================
// Conversion: RequestParts/ResponseParts -> http types
// ============================================================================

impl TryFrom<RequestParts> for http::request::Parts {
	type Error = http::Error;

	fn try_from(parts: RequestParts) -> Result<Self, Self::Error> {
		let method: http::Method = parts.method.into();

		// Build URI from path and params
		let uri_str = parts.parts.uri();

		let mut builder = http::Request::builder()
			.method(method)
			.uri(&uri_str)
			.version(http_ext::parse_version(&parts.parts.version));

		// Add headers
		if let Ok(header_map) = multimap_to_header_map(&parts.parts.headers) {
			for (key, value) in header_map.iter() {
				builder = builder.header(key, value);
			}
		}

		let (http_parts, _) = builder.body(())?.into_parts();
		Ok(http_parts)
	}
}

impl TryFrom<ResponseParts> for http::response::Parts {
	type Error = http::Error;

	fn try_from(parts: ResponseParts) -> Result<Self, Self::Error> {
		let mut builder = http::Response::builder()
			.status(parts.status)
			.version(http_ext::parse_version(&parts.parts.version));

		// Add headers
		if let Ok(header_map) = multimap_to_header_map(&parts.parts.headers) {
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
	fn parts_default() {
		let parts = Parts::default();
		parts.path().xpect_empty();
		parts.uri().xpect_eq("/");
		parts.scheme().clone().xpect_eq(Scheme::None);
		parts.version().xpect_eq("1.1");
	}

	#[test]
	fn parts_builder() {
		let parts = PartsBuilder::new()
			.scheme(Scheme::Http)
			.authority("example.com")
			.path_str("/api/users/123")
			.param("limit", "10")
			.header("content-type", "application/json")
			.build();

		parts.scheme().clone().xpect_eq(Scheme::Http);
		parts.authority().xpect_eq("example.com");
		parts.path().xpect_eq(vec![
			"api".to_string(),
			"users".to_string(),
			"123".to_string(),
		]);
		parts.get_param("limit").unwrap().xpect_eq("10");
		parts
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/json");
	}

	#[test]
	fn parts_builder_request_parts() {
		let parts = PartsBuilder::new()
			.path_str("/api/users")
			.param("page", "1")
			.build_request_parts(HttpMethod::Post);

		(*parts.method()).xpect_eq(HttpMethod::Post);
		parts
			.path()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		parts.get_param("page").unwrap().xpect_eq("1");
	}

	#[test]
	fn parts_builder_response_parts() {
		let parts = PartsBuilder::new()
			.header("content-type", "text/html")
			.build_response_parts(StatusCode::CREATED);

		parts.status().xpect_eq(StatusCode::CREATED);
		parts
			.get_header("content-type")
			.unwrap()
			.xpect_eq("text/html");
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
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/json");
	}

	#[test]
	fn from_http_response_parts() {
		let http_parts = http::Response::builder()
			.status(StatusCode::CREATED)
			.header("content-type", "application/json")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		let parts = ResponseParts::from(http_parts);

		parts.status().xpect_eq(StatusCode::CREATED);
		parts
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/json");
	}

	#[test]
	fn from_cli_args() {
		let cli = CliArgs::parse("users list --limit 10 --verbose");
		let parts = RequestParts::from(cli);

		parts.scheme().clone().xpect_eq(Scheme::Cli);
		(*parts.method()).xpect_eq(HttpMethod::Get);
		parts
			.path()
			.xpect_eq(vec!["users".to_string(), "list".to_string()]);
		parts.get_param("limit").unwrap().xpect_eq("10");
		parts.has_param("verbose").xpect_true();
		// parts.version().xpect_eq("0.1.0");
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
		let parts = PartsBuilder::new().path_str("/api/users/123").build();
		parts.path_string().xpect_eq("/api/users/123");

		let empty_parts = Parts::default();
		empty_parts.path_string().xpect_eq("/");
	}

	#[test]
	fn query_string() {
		let parts = PartsBuilder::new()
			.param("limit", "10")
			.param("offset", "20")
			.build();
		let query = parts.query_string();
		// Order may vary, so check both params are present
		(&query).xpect_contains("limit=10");
		(&query).xpect_contains("offset=20");
	}

	#[test]
	fn uri_construction() {
		let parts = PartsBuilder::new()
			.path_str("/api/users")
			.param("page", "1")
			.build();
		let uri = parts.uri();
		(&uri).xpect_starts_with("/api/users?");
		(&uri).xpect_contains("page=1");
	}

	#[test]
	fn path_segments() {
		let parts = PartsBuilder::new().path_str("/api/users/123").build();

		parts.first_segment().unwrap().xpect_eq("api");
		parts.last_segment().unwrap().xpect_eq("123");
		parts.path_from(1).xpect_eq(["users", "123"]);
		parts.path_from(10).len().xpect_eq(0);
	}

	#[test]
	fn split_path_handles_edge_cases() {
		split_path("").xpect_empty();
		split_path("/").xpect_empty();
		split_path("//").xpect_empty();
		split_path("/a//b/").xpect_eq(vec!["a".to_string(), "b".to_string()]);
	}

	#[test]
	fn scheme_parsing() {
		Scheme::from_str("http").xpect_eq(Scheme::Http);
		Scheme::from_str("HTTPS").xpect_eq(Scheme::Https);
		Scheme::from_str("cli").xpect_eq(Scheme::Cli);
		Scheme::from_str("").xpect_eq(Scheme::None);
		Scheme::from_str("custom")
			.xpect_eq(Scheme::Other("custom".to_string()));
	}

	#[test]
	fn request_parts_to_http() {
		let parts = PartsBuilder::new()
			.path_str("/api/users")
			.param("limit", "10")
			.header("content-type", "application/json")
			.build_request_parts(HttpMethod::Post);

		let http_parts: http::request::Parts = parts.try_into().unwrap();

		http_parts.method.xpect_eq(http::Method::POST);
		http_parts.uri.path().xpect_eq("/api/users");
		http_parts.uri.query().unwrap().xpect_eq("limit=10");
	}

	#[test]
	fn response_parts_to_http() {
		let parts = PartsBuilder::new()
			.header("content-type", "application/json")
			.build_response_parts(StatusCode::CREATED);

		let http_parts: http::response::Parts = parts.try_into().unwrap();

		http_parts.status.xpect_eq(StatusCode::CREATED);
	}

	#[test]
	fn request_parts_deref() {
		let parts = RequestParts::get("/api/users");
		// Access Parts methods via Deref
		parts.path().len().xpect_eq(2);
		parts.path_string().xpect_eq("/api/users");
	}

	#[test]
	fn response_parts_deref() {
		let mut parts = ResponseParts::ok();
		// Access Parts methods via DerefMut
		parts
			.parts_mut()
			.headers
			.insert("x-custom".to_string(), "value".to_string());
		parts.get_header("x-custom").unwrap().xpect_eq("value");
	}

	#[test]
	fn has_body_detection() {
		let mut parts = Parts::default();
		parts.has_body().xpect_false();

		parts.insert_header("content-length", "5");
		parts.has_body().xpect_true();

		let mut parts2 = Parts::default();
		parts2.insert_header("transfer-encoding", "chunked");
		parts2.has_body().xpect_true();
	}
}
