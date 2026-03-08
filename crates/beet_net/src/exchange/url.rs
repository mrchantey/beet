//! Application-friendly URL type for routing and request construction.
//!
//! [`Url`] provides a structured representation of URLs that prioritizes
//! ease of use over zero-copy performance. When parsing, the double slashes
//! after the scheme may be omitted (ie `http:example.com`), but they are
//! always included when rendering to string.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! let url = Url::parse("https://example.com/api/users?limit=10#results");
//! assert_eq!(url.scheme(), &Scheme::Https);
//! assert_eq!(url.authority(), Some("example.com"));
//! assert_eq!(url.path(), &["api", "users"]);
//! assert_eq!(url.get_param("limit"), Some("10"));
//! assert_eq!(url.fragment(), Some("results"));
//! assert_eq!(url.to_string(), "https://example.com/api/users?limit=10#results");
//! ```

use beet_core::prelude::*;

/// The transport scheme of a URL.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub enum Scheme {
	/// No scheme specified, ie an absolute or relative path.
	#[default]
	None,
	/// `http`
	Http,
	/// `https`
	Https,
	/// `file`
	File,
	/// `ws`
	Ws,
	/// `wss`
	Wss,
	/// A scheme not covered by the named variants.
	Other(String),
}

impl Scheme {
	/// Parse a scheme from a string.
	pub fn from_str(scheme: &str) -> Self {
		match scheme.to_ascii_lowercase().as_str() {
			"http" => Self::Http,
			"https" => Self::Https,
			"file" => Self::File,
			"ws" => Self::Ws,
			"wss" => Self::Wss,
			"" => Self::None,
			other => Self::Other(other.to_string()),
		}
	}

	/// The canonical string representation of the scheme.
	pub fn as_str(&self) -> &str {
		match self {
			Self::None => "",
			Self::Http => "http",
			Self::Https => "https",
			Self::File => "file",
			Self::Ws => "ws",
			Self::Wss => "wss",
			Self::Other(scheme) => scheme.as_str(),
		}
	}

	/// Whether this is an HTTP-based scheme.
	pub fn is_http(&self) -> bool { matches!(self, Self::Http | Self::Https) }

	/// Whether this is a WebSocket scheme.
	pub fn is_ws(&self) -> bool { matches!(self, Self::Ws | Self::Wss) }

	/// Whether this scheme uses TLS.
	pub fn is_secure(&self) -> bool { matches!(self, Self::Https | Self::Wss) }
}

impl std::fmt::Display for Scheme {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(formatter, "{}", self.as_str())
	}
}

#[cfg(feature = "http")]
impl From<&http::uri::Scheme> for Scheme {
	fn from(scheme: &http::uri::Scheme) -> Self {
		Self::from_str(scheme.as_str())
	}
}

#[cfg(feature = "http")]
impl From<Option<&http::uri::Scheme>> for Scheme {
	fn from(scheme: Option<&http::uri::Scheme>) -> Self {
		scheme.map(Self::from).unwrap_or(Self::None)
	}
}


/// An application-friendly URL type.
///
/// Stores the components of a URL in parsed form for easy manipulation.
/// When parsing, the `://` separator is flexible — `http:example.com`
/// is treated the same as `http://example.com`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Url {
	scheme: Scheme,
	authority: Option<String>,
	path: Vec<String>,
	params: MultiMap<String, String>,
	fragment: Option<String>,
}

impl Url {
	/// Parse a URL string.
	///
	/// Accepts full URLs (`https://example.com/path`), scheme-relative
	/// URLs without the double slash (`http:example.com/path`), and
	/// bare paths (`/api/users?q=1`).
	pub fn parse(input: impl AsRef<str>) -> Self {
		let input = input.as_ref();

		// Split off fragment first
		let (before_fragment, fragment) = match input.split_once('#') {
			Some((before, frag)) if !frag.is_empty() => {
				(before, Some(frag.to_string()))
			}
			Some((before, _)) => (before, None),
			None => (input, None),
		};

		// Split off query string
		let (before_query, query_str) = match before_fragment.split_once('?') {
			Some((before, query)) => (before, Some(query)),
			None => (before_fragment, None),
		};

		let params = query_str.map(parse_query_string).unwrap_or_default();

		// Try to detect a scheme
		let (scheme, rest) = match before_query.split_once(':') {
			Some((maybe_scheme, rest))
				if !maybe_scheme.is_empty()
					&& maybe_scheme.bytes().all(|byte| {
						byte.is_ascii_alphanumeric()
							|| byte == b'+' || byte == b'-'
							|| byte == b'.'
					}) =>
			{
				let scheme = Scheme::from_str(maybe_scheme);
				// Strip optional leading `//`
				let rest = rest.strip_prefix("//").unwrap_or(rest);
				(scheme, rest)
			}
			_ => (Scheme::None, before_query),
		};

		// For scheme-bearing URLs, extract authority (host[:port])
		let (authority, path_str) = if scheme != Scheme::None {
			// Authority ends at the first `/` (or end of string)
			match rest.split_once('/') {
				Some((auth, path)) if !auth.is_empty() => {
					(Some(auth.to_string()), format!("/{path}"))
				}
				_ if !rest.is_empty() && !rest.starts_with('/') => {
					// Entire rest is the authority with no path
					(Some(rest.to_string()), String::new())
				}
				_ => (None, rest.to_string()),
			}
		} else {
			(None, rest.to_string())
		};

		let path = split_path(&path_str);

		Self {
			scheme,
			authority,
			path,
			params,
			fragment,
		}
	}

	/// Create a URL from individual components.
	pub fn new(
		scheme: Scheme,
		authority: Option<String>,
		path: Vec<String>,
		params: MultiMap<String, String>,
		fragment: Option<String>,
	) -> Self {
		Self {
			scheme,
			authority,
			path,
			params,
			fragment,
		}
	}

	/// The scheme of the URL.
	pub fn scheme(&self) -> &Scheme { &self.scheme }

	/// Set the scheme.
	pub fn with_scheme(mut self, scheme: Scheme) -> Self {
		self.scheme = scheme;
		self
	}

	/// The authority (host and optional port), if present.
	pub fn authority(&self) -> Option<&str> { self.authority.as_deref() }

	/// Set the authority.
	pub fn with_authority(mut self, authority: impl Into<String>) -> Self {
		self.authority = Some(authority.into());
		self
	}

	/// The path segments.
	pub fn path(&self) -> &Vec<String> { &self.path }

	/// A mutable reference to the path segments.
	pub fn path_mut(&mut self) -> &mut Vec<String> { &mut self.path }

	/// Set the path segments.
	pub fn with_path(mut self, path: Vec<String>) -> Self {
		self.path = path;
		self
	}

	/// All query parameters.
	pub fn params(&self) -> &MultiMap<String, String> { &self.params }

	/// A mutable reference to the query parameters.
	pub fn params_mut(&mut self) -> &mut MultiMap<String, String> {
		&mut self.params
	}

	/// Get the first value for a query parameter.
	pub fn get_param(&self, key: &str) -> Option<&str> {
		self.params
			.get_vec(key)
			.and_then(|vals| vals.first().map(|val| val.as_str()))
	}

	/// Check if a query parameter exists.
	pub fn has_param(&self, key: &str) -> bool { self.params.contains_key(key) }

	/// Add a query parameter, returning self for chaining.
	pub fn with_param(
		mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) -> Self {
		self.params.insert(key.into(), value.into());
		self
	}

	/// Add a flag parameter (key with no value).
	pub fn with_flag(mut self, key: impl Into<String>) -> Self {
		self.params.insert_key(key.into());
		self
	}

	/// The fragment identifier, if present.
	pub fn fragment(&self) -> Option<&str> { self.fragment.as_deref() }

	/// Set the fragment identifier.
	pub fn with_fragment(mut self, fragment: impl Into<String>) -> Self {
		self.fragment = Some(fragment.into());
		self
	}

	/// The path as a string with leading `/`.
	pub fn path_string(&self) -> String {
		if self.path.is_empty() {
			"/".to_string()
		} else {
			format!("/{}", self.path.join("/"))
		}
	}

	/// The query string built from parameters.
	pub fn query_string(&self) -> String { build_query_string(&self.params) }

	/// The first path segment, if any.
	pub fn first_segment(&self) -> Option<&str> {
		self.path.first().map(|seg| seg.as_str())
	}

	/// The last path segment, if any.
	pub fn last_segment(&self) -> Option<&str> {
		self.path.last().map(|seg| seg.as_str())
	}

	/// Path segments starting from the given index.
	pub fn path_from(&self, index: usize) -> &[String] {
		if index >= self.path.len() {
			&[]
		} else {
			&self.path[index..]
		}
	}
}

impl std::fmt::Display for Url {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let path = self.path_string();
		let query = self.query_string();

		match (&self.scheme, &self.authority) {
			(Scheme::None, _) => write!(formatter, "{path}")?,
			(scheme, Some(auth)) => {
				write!(formatter, "{scheme}://{auth}{path}")?
			}
			(scheme, None) => write!(formatter, "{scheme}://{path}")?,
		};

		if !query.is_empty() {
			write!(formatter, "?{query}")?;
		}
		if let Some(frag) = &self.fragment {
			write!(formatter, "#{frag}")?;
		}

		Ok(())
	}
}

impl From<&str> for Url {
	fn from(value: &str) -> Self { Url::parse(value) }
}

impl From<String> for Url {
	fn from(value: String) -> Self { Url::parse(value) }
}

// ============================================================================
// Shared parsing helpers (also used by parts.rs)
// ============================================================================

/// Parse a query string into a [`MultiMap`].
pub(crate) fn parse_query_string(query: &str) -> MultiMap<String, String> {
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

/// Split a path string into segments, filtering empty segments.
pub(crate) fn split_path(path: &str) -> Vec<String> {
	path.split('/')
		.filter(|segment| !segment.is_empty())
		.map(|segment| segment.to_string())
		.collect()
}

/// Build a query string from a [`MultiMap`].
pub(crate) fn build_query_string(params: &MultiMap<String, String>) -> String {
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

/// Convert an [`http::HeaderMap`] to a [`super::HeaderMap`],
/// with all keys normalized to kebab-case.
#[cfg(feature = "http")]
pub(crate) fn http_header_map_to_header_map(
	map: &http::HeaderMap,
) -> super::HeaderMap {
	let mut header_map = super::HeaderMap::new();
	for (key, value) in map.iter() {
		let value = value.to_str().unwrap_or("<opaque-bytes>").to_string();
		header_map.set_raw(key.as_str(), value);
	}
	header_map
}

/// Convert a [`super::HeaderMap`] back to [`http::HeaderMap`].
#[cfg(feature = "http")]
pub(crate) fn header_map_to_http(
	headers: &super::HeaderMap,
) -> Result<http::HeaderMap, http::header::InvalidHeaderValue> {
	use std::str::FromStr;
	let mut http_headers = http::HeaderMap::new();
	for (key, values) in headers.iter_all() {
		let header_name = http::header::HeaderName::from_str(key)
			.unwrap_or_else(|_| {
				http::header::HeaderName::from_static("x-invalid")
			});
		for value in values {
			http_headers.append(
				header_name.clone(),
				http::header::HeaderValue::from_str(value)?,
			);
		}
	}
	Ok(http_headers)
}


#[cfg(test)]
mod test {
	use super::*;

	// -- Scheme tests --

	#[test]
	fn scheme_parsing() {
		Scheme::from_str("http").xpect_eq(Scheme::Http);
		Scheme::from_str("HTTPS").xpect_eq(Scheme::Https);
		Scheme::from_str("file").xpect_eq(Scheme::File);
		Scheme::from_str("ws").xpect_eq(Scheme::Ws);
		Scheme::from_str("wss").xpect_eq(Scheme::Wss);
		Scheme::from_str("").xpect_eq(Scheme::None);
		Scheme::from_str("custom")
			.xpect_eq(Scheme::Other("custom".to_string()));
	}

	#[test]
	fn scheme_display() {
		Scheme::Http.to_string().xpect_eq("http");
		Scheme::None.to_string().xpect_eq("");
	}

	#[test]
	fn scheme_is_http() {
		Scheme::Http.is_http().xpect_true();
		Scheme::Https.is_http().xpect_true();
		Scheme::Ws.is_http().xpect_false();
	}

	#[test]
	fn scheme_is_ws() {
		Scheme::Ws.is_ws().xpect_true();
		Scheme::Wss.is_ws().xpect_true();
		Scheme::Http.is_ws().xpect_false();
	}

	#[test]
	fn scheme_is_secure() {
		Scheme::Https.is_secure().xpect_true();
		Scheme::Wss.is_secure().xpect_true();
		Scheme::Http.is_secure().xpect_false();
	}

	// -- Url parsing tests --

	#[test]
	fn parse_full_url() {
		let url = Url::parse("https://example.com/api/users?limit=10#results");
		url.scheme().clone().xpect_eq(Scheme::Https);
		url.authority().unwrap().xpect_eq("example.com");
		url.path()
			.clone()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		url.get_param("limit").unwrap().xpect_eq("10");
		url.fragment().unwrap().xpect_eq("results");
	}

	#[test]
	fn parse_without_double_slash() {
		let url = Url::parse("http:example.com/api/users");
		url.scheme().clone().xpect_eq(Scheme::Http);
		url.authority().unwrap().xpect_eq("example.com");
		url.path()
			.clone()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
	}

	#[test]
	fn parse_bare_path() {
		let url = Url::parse("/api/users?page=1");
		url.scheme().clone().xpect_eq(Scheme::None);
		url.authority().xpect_none();
		url.path()
			.clone()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		url.get_param("page").unwrap().xpect_eq("1");
	}

	#[test]
	fn parse_authority_only() {
		let url = Url::parse("https://example.com");
		url.scheme().clone().xpect_eq(Scheme::Https);
		url.authority().unwrap().xpect_eq("example.com");
		url.path().xpect_empty();
	}

	#[test]
	fn parse_with_port() {
		let url = Url::parse("http://localhost:8080/api");
		url.authority().unwrap().xpect_eq("localhost:8080");
		url.path().clone().xpect_eq(vec!["api".to_string()]);
	}

	#[test]
	fn parse_file_scheme() {
		let url = Url::parse("file:///home/user/doc.txt");
		url.scheme().clone().xpect_eq(Scheme::File);
		url.authority().xpect_none();
		url.path().clone().xpect_eq(vec![
			"home".to_string(),
			"user".to_string(),
			"doc.txt".to_string(),
		]);
	}

	#[test]
	fn parse_empty_fragment() {
		let url = Url::parse("/path#");
		url.fragment().xpect_none();
	}

	// -- Display / roundtrip tests --

	#[test]
	fn display_full_url() {
		let url = Url::parse("https://example.com/api/users?limit=10#results");
		url.to_string()
			.xpect_eq("https://example.com/api/users?limit=10#results");
	}

	#[test]
	fn display_normalizes_double_slash() {
		let url = Url::parse("http:example.com/path");
		(&url.to_string()).xpect_starts_with("http://example.com/path");
	}

	#[test]
	fn display_bare_path() {
		let url = Url::parse("/api/users");
		url.to_string().xpect_eq("/api/users");
	}

	#[test]
	fn display_empty_path() {
		let url = Url::default();
		url.to_string().xpect_eq("/");
	}

	// -- Builder tests --

	#[test]
	fn builder_chaining() {
		let url = Url::default()
			.with_scheme(Scheme::Https)
			.with_authority("example.com")
			.with_path(vec!["api".to_string()])
			.with_param("key", "val")
			.with_fragment("top");
		url.to_string()
			.xpect_eq("https://example.com/api?key=val#top");
	}

	// -- Helper tests --

	#[test]
	fn split_path_handles_edge_cases() {
		split_path("").xpect_empty();
		split_path("/").xpect_empty();
		split_path("//").xpect_empty();
		split_path("/a//b/").xpect_eq(vec!["a".to_string(), "b".to_string()]);
	}

	#[test]
	fn path_string() {
		let url = Url::parse("/api/users/123");
		url.path_string().xpect_eq("/api/users/123");

		let empty = Url::default();
		empty.path_string().xpect_eq("/");
	}

	#[test]
	fn query_string() {
		let url = Url::parse("/?limit=10&offset=20");
		let query = url.query_string();
		(&query).xpect_contains("limit=10");
		(&query).xpect_contains("offset=20");
	}

	#[test]
	fn path_segments() {
		let url = Url::parse("/api/users/123");
		url.first_segment().unwrap().xpect_eq("api");
		url.last_segment().unwrap().xpect_eq("123");
		url.path_from(1).xpect_eq(["users", "123"]);
		url.path_from(10).len().xpect_eq(0);
	}

	#[test]
	fn with_flag() {
		let url = Url::default().with_flag("verbose");
		url.has_param("verbose").xpect_true();
	}

	#[test]
	fn from_str_impl() {
		let url: Url = "https://example.com/path".into();
		url.scheme().clone().xpect_eq(Scheme::Https);
	}
}
