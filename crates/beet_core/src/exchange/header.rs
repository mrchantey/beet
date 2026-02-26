//! Typed header marker structs for use with [`super::HeaderMap::get`] and [`super::HeaderMap::set`].
//!
//! Import this module as `headers` for ergonomic typed access:
//!
//! ```
//! # use beet_core::prelude::*;
//! # use beet_core::exchange::headers;
//! let mut map = HeaderMap::new();
//! map.set::<headers::ContentType>(&MimeType::Json);
//! let mime = map.get::<headers::ContentType>().unwrap().unwrap();
//! assert_eq!(mime, MimeType::Json);
//! ```

use crate::prelude::*;

// ============================================================================
// ContentType
// ============================================================================

/// Typed `Content-Type` header, parsed as a [`MimeType`].
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut headers = HeaderMap::new();
/// headers.set_raw("content-type", "application/json");
/// let mime: MimeType = headers.get::<headers::ContentType>().unwrap().unwrap();
/// assert_eq!(mime, MimeType::Json);
/// ```
pub struct ContentType;

impl Header for ContentType {
	type Value = MimeType;
	const KEY: &'static str = "content-type";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.map(|val| MimeType::from_content_type(val))
			.ok_or_else(|| bevyhow!("content-type header has no value"))
	}

	fn serialize(value: &MimeType) -> Vec<String> {
		vec![value.as_str().to_string()]
	}
}

// ============================================================================
// Accept
// ============================================================================

/// Typed `Accept` header, parsed as a list of [`MimeType`] ordered by quality.
///
/// Quality scores (eg `text/html;q=0.9`) are used for ordering but stripped
/// from the resulting values. Higher quality types come first.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set_raw("accept", "text/html;q=0.9, application/json");
/// let types: Vec<MimeType> = map.get::<headers::Accept>().unwrap().unwrap();
/// assert_eq!(types, vec![MimeType::Json, MimeType::Html]);
/// ```
pub struct Accept;

struct QualityMime {
	mime: MimeType,
	quality: f32,
}

impl Header for Accept {
	type Value = Vec<MimeType>;
	const KEY: &'static str = "accept";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		let mut entries: Vec<QualityMime> = Vec::new();
		for value in values {
			for part in value.split(',') {
				let part = part.trim();
				if part.is_empty() {
					continue;
				}
				let (mime_str, quality) = parse_quality(part);
				entries.push(QualityMime {
					mime: MimeType::from_content_type(mime_str),
					quality,
				});
			}
		}
		// Stable sort preserves insertion order for equal quality values
		entries.sort_by(|left, right| {
			right
				.quality
				.partial_cmp(&left.quality)
				.unwrap_or(core::cmp::Ordering::Equal)
		});
		entries
			.into_iter()
			.map(|entry| entry.mime)
			.collect::<Vec<_>>()
			.xok()
	}

	fn serialize(value: &Vec<MimeType>) -> Vec<String> {
		vec![
			value
				.iter()
				.map(|mime| mime.as_str().to_string())
				.collect::<Vec<_>>()
				.join(", "),
		]
	}
}

/// Parse a quality parameter from a MIME type string.
/// Returns `(mime_str, quality)` where quality defaults to `1.0`.
fn parse_quality(part: &str) -> (&str, f32) {
	let mut segments = part.splitn(2, ';');
	let mime_str = segments.next().unwrap_or(part).trim();
	let quality = segments
		.next()
		.and_then(|params| {
			params.split(';').find_map(|param| {
				let param = param.trim();
				param
					.strip_prefix("q=")
					.and_then(|val| val.trim().parse::<f32>().ok())
			})
		})
		.unwrap_or(1.0);
	(mime_str, quality)
}

// ============================================================================
// Location
// ============================================================================

/// Typed `Location` header, used for redirects.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::Location>(&"/new/path".to_string());
/// assert_eq!(map.get::<headers::Location>().unwrap().unwrap(), "/new/path");
/// ```
pub struct Location;

impl Header for Location {
	type Value = String;
	const KEY: &'static str = "location";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.cloned()
			.ok_or_else(|| bevyhow!("location header has no value"))
	}

	fn serialize(value: &String) -> Vec<String> { vec![value.clone()] }
}

// ============================================================================
// Authorization
// ============================================================================

/// Typed `Authorization` header value.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::Authorization>(&headers::Authorization::bearer("abc123"));
/// let auth = map.get::<headers::Authorization>().unwrap().unwrap();
/// assert_eq!(auth, headers::Authorization::Bearer("abc123".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authorization {
	/// `Bearer <token>`
	Bearer(String),
	/// `Basic <credentials>`
	Basic(String),
	/// Any other scheme.
	Other(String),
}

impl Authorization {
	/// Create a `Bearer` token authorization value.
	pub fn bearer(token: impl Into<String>) -> Self {
		Self::Bearer(token.into())
	}
}

impl core::fmt::Display for Authorization {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Bearer(token) => write!(f, "Bearer {token}"),
			Self::Basic(creds) => write!(f, "Basic {creds}"),
			Self::Other(raw) => write!(f, "{raw}"),
		}
	}
}

impl Header for Authorization {
	type Value = Authorization;
	const KEY: &'static str = "authorization";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		let raw = values
			.first()
			.ok_or_else(|| bevyhow!("authorization header has no value"))?;
		if let Some(token) = raw.strip_prefix("Bearer ") {
			Authorization::Bearer(token.to_string()).xok()
		} else if let Some(creds) = raw.strip_prefix("Basic ") {
			Authorization::Basic(creds.to_string()).xok()
		} else {
			Authorization::Other(raw.clone()).xok()
		}
	}

	fn serialize(value: &Authorization) -> Vec<String> {
		vec![value.to_string()]
	}
}

// ============================================================================
// CacheControl
// ============================================================================

/// Typed `Cache-Control` header.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::CacheControl>(&"no-cache, no-store".to_string());
/// assert_eq!(map.get::<headers::CacheControl>().unwrap().unwrap(), "no-cache, no-store");
/// ```
pub struct CacheControl;

impl Header for CacheControl {
	type Value = String;
	const KEY: &'static str = "cache-control";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.cloned()
			.ok_or_else(|| bevyhow!("cache-control header has no value"))
	}

	fn serialize(value: &String) -> Vec<String> { vec![value.clone()] }
}

// ============================================================================
// ContentLength
// ============================================================================

/// Typed `Content-Length` header.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::ContentLength>(&42u64);
/// assert_eq!(map.get::<headers::ContentLength>().unwrap().unwrap(), 42u64);
/// ```
pub struct ContentLength;

impl Header for ContentLength {
	type Value = u64;
	const KEY: &'static str = "content-length";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.ok_or_else(|| bevyhow!("content-length header has no value"))?
			.parse::<u64>()
			.map_err(|err| bevyhow!("invalid content-length: {err}"))
	}

	fn serialize(value: &u64) -> Vec<String> { vec![value.to_string()] }
}

// ============================================================================
// Origin
// ============================================================================

/// Typed `Origin` header.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::Origin>(&"https://example.com".to_string());
/// assert_eq!(map.get::<headers::Origin>().unwrap().unwrap(), "https://example.com");
/// ```
pub struct Origin;

impl Header for Origin {
	type Value = String;
	const KEY: &'static str = "origin";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.cloned()
			.ok_or_else(|| bevyhow!("origin header has no value"))
	}

	fn serialize(value: &String) -> Vec<String> { vec![value.clone()] }
}

// ============================================================================
// AccessControlAllowOrigin
// ============================================================================

/// Typed `Access-Control-Allow-Origin` header.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::AccessControlAllowOrigin>(&"*".to_string());
/// assert_eq!(map.get::<headers::AccessControlAllowOrigin>().unwrap().unwrap(), "*");
/// ```
pub struct AccessControlAllowOrigin;

impl Header for AccessControlAllowOrigin {
	type Value = String;
	const KEY: &'static str = "access-control-allow-origin";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values.first().cloned().ok_or_else(|| {
			bevyhow!("access-control-allow-origin header has no value")
		})
	}

	fn serialize(value: &String) -> Vec<String> { vec![value.clone()] }
}

// ============================================================================
// AccessControlAllowHeaders
// ============================================================================

/// Typed `Access-Control-Allow-Headers` header.
pub struct AccessControlAllowHeaders;

impl Header for AccessControlAllowHeaders {
	type Value = String;
	const KEY: &'static str = "access-control-allow-headers";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values.first().cloned().ok_or_else(|| {
			bevyhow!("access-control-allow-headers header has no value")
		})
	}

	fn serialize(value: &String) -> Vec<String> { vec![value.clone()] }
}

// ============================================================================
// AccessControlMaxAge
// ============================================================================

/// Typed `Access-Control-Max-Age` header, value in seconds.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::AccessControlMaxAge>(&3600u32);
/// assert_eq!(map.get::<headers::AccessControlMaxAge>().unwrap().unwrap(), 3600u32);
/// ```
pub struct AccessControlMaxAge;

impl Header for AccessControlMaxAge {
	type Value = u32;
	const KEY: &'static str = "access-control-max-age";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.ok_or_else(|| {
				bevyhow!("access-control-max-age header has no value")
			})?
			.parse::<u32>()
			.map_err(|err| bevyhow!("invalid access-control-max-age: {err}"))
	}

	fn serialize(value: &u32) -> Vec<String> { vec![value.to_string()] }
}

// ============================================================================
// TransferEncoding
// ============================================================================

/// Value for the `Transfer-Encoding` header.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set::<headers::TransferEncoding>(&headers::TransferEncodingValue::Chunked);
/// assert_eq!(
///     map.get::<headers::TransferEncoding>().unwrap().unwrap(),
///     headers::TransferEncodingValue::Chunked,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferEncodingValue {
	/// `chunked` — body is sent in a series of chunks.
	Chunked,
	/// `compress` — LZW compression.
	Compress,
	/// `deflate` — zlib deflate compression.
	Deflate,
	/// `gzip` — GNU zip compression.
	Gzip,
	/// Any other transfer-encoding value.
	Other(String),
}

impl TransferEncodingValue {
	/// Returns the canonical string representation.
	pub fn as_str(&self) -> &str {
		match self {
			Self::Chunked => "chunked",
			Self::Compress => "compress",
			Self::Deflate => "deflate",
			Self::Gzip => "gzip",
			Self::Other(val) => val.as_str(),
		}
	}
}

impl core::fmt::Display for TransferEncodingValue {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl From<&str> for TransferEncodingValue {
	fn from(val: &str) -> Self {
		match val {
			"chunked" => Self::Chunked,
			"compress" => Self::Compress,
			"deflate" => Self::Deflate,
			"gzip" => Self::Gzip,
			other => Self::Other(other.to_string()),
		}
	}
}

/// Typed `Transfer-Encoding` header.
pub struct TransferEncoding;

impl Header for TransferEncoding {
	type Value = TransferEncodingValue;
	const KEY: &'static str = "transfer-encoding";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.map(|val| TransferEncodingValue::from(val.as_str()))
			.ok_or_else(|| bevyhow!("transfer-encoding header has no value"))
	}

	fn serialize(value: &TransferEncodingValue) -> Vec<String> {
		vec![value.as_str().to_string()]
	}
}

// ============================================================================
// SetCookie / Cookie
// ============================================================================

/// Typed `Set-Cookie` header — returns all values since there may be multiple.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_core::exchange::headers;
/// let mut map = HeaderMap::new();
/// map.set_raw("set-cookie", "a=1");
/// map.set_raw("set-cookie", "b=2");
/// let cookies = map.get::<headers::SetCookie>().unwrap().unwrap();
/// assert_eq!(cookies, vec!["a=1".to_string(), "b=2".to_string()]);
/// ```
pub struct SetCookie;

impl Header for SetCookie {
	type Value = Vec<String>;
	const KEY: &'static str = "set-cookie";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		if values.is_empty() {
			bevybail!("set-cookie header has no values");
		}
		values.clone().xok()
	}

	fn serialize(value: &Vec<String>) -> Vec<String> { value.clone() }
}

/// Typed `Cookie` request header — returns all values.
pub struct Cookie;

impl Header for Cookie {
	type Value = Vec<String>;
	const KEY: &'static str = "cookie";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		if values.is_empty() {
			bevybail!("cookie header has no values");
		}
		values.clone().xok()
	}

	fn serialize(value: &Vec<String>) -> Vec<String> { value.clone() }
}

// ============================================================================
// Pragma / Expires
// ============================================================================

/// Typed `Pragma` header.
pub struct Pragma;

impl Header for Pragma {
	type Value = String;
	const KEY: &'static str = "pragma";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.cloned()
			.ok_or_else(|| bevyhow!("pragma header has no value"))
	}

	fn serialize(value: &String) -> Vec<String> { vec![value.clone()] }
}

/// Typed `Expires` header.
pub struct Expires;

impl Header for Expires {
	type Value = String;
	const KEY: &'static str = "expires";

	fn parse(values: &Vec<String>) -> Result<Self::Value> {
		values
			.first()
			.cloned()
			.ok_or_else(|| bevyhow!("expires header has no value"))
	}

	fn serialize(value: &String) -> Vec<String> { vec![value.clone()] }
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn content_type_roundtrip() {
		let mut map = HeaderMap::new();
		map.set::<ContentType>(&MimeType::Json);
		map.get::<ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MimeType::Json);
	}

	#[test]
	fn accept_quality_ordering() {
		let mut map = HeaderMap::new();
		map.set_raw("accept", "text/html;q=0.9, application/json");
		let types = map.get::<Accept>().unwrap().unwrap();
		types[0].clone().xpect_eq(MimeType::Json);
		types[1].clone().xpect_eq(MimeType::Html);
	}

	#[test]
	fn accept_equal_quality_preserves_order() {
		let mut map = HeaderMap::new();
		map.set_raw("accept", "text/html, application/json");
		let types = map.get::<Accept>().unwrap().unwrap();
		types[0].clone().xpect_eq(MimeType::Html);
		types[1].clone().xpect_eq(MimeType::Json);
	}

	#[test]
	fn location_roundtrip() {
		let mut map = HeaderMap::new();
		map.set::<Location>(&"/redirect".to_string());
		map.get::<Location>()
			.unwrap()
			.unwrap()
			.xpect_eq("/redirect");
	}

	#[test]
	fn authorization_bearer() {
		let mut map = HeaderMap::new();
		map.set::<Authorization>(&Authorization::bearer("tok123"));
		let auth = map.get::<Authorization>().unwrap().unwrap();
		auth.xpect_eq(Authorization::Bearer("tok123".to_string()));
	}

	#[test]
	fn authorization_basic() {
		let mut map = HeaderMap::new();
		map.set_raw("authorization", "Basic dXNlcjpwYXNz");
		let auth = map.get::<Authorization>().unwrap().unwrap();
		auth.xpect_eq(Authorization::Basic("dXNlcjpwYXNz".to_string()));
	}

	#[test]
	fn authorization_other() {
		let mut map = HeaderMap::new();
		map.set_raw("authorization", "Digest realm=\"test\"");
		let auth = map.get::<Authorization>().unwrap().unwrap();
		auth.xpect_eq(Authorization::Other(
			"Digest realm=\"test\"".to_string(),
		));
	}

	#[test]
	fn content_length_roundtrip() {
		let mut map = HeaderMap::new();
		map.set::<ContentLength>(&1024u64);
		map.get::<ContentLength>()
			.unwrap()
			.unwrap()
			.xpect_eq(1024u64);
	}

	#[test]
	fn content_length_invalid() {
		let mut map = HeaderMap::new();
		map.set_raw("content-length", "not-a-number");
		map.get::<ContentLength>().unwrap().xpect_err();
	}

	#[test]
	fn origin_roundtrip() {
		let mut map = HeaderMap::new();
		map.set::<Origin>(&"https://example.com".to_string());
		map.get::<Origin>()
			.unwrap()
			.unwrap()
			.xpect_eq("https://example.com");
	}

	#[test]
	fn access_control_allow_origin_wildcard() {
		let mut map = HeaderMap::new();
		map.set::<AccessControlAllowOrigin>(&"*".to_string());
		map.get::<AccessControlAllowOrigin>()
			.unwrap()
			.unwrap()
			.xpect_eq("*");
	}

	#[test]
	fn access_control_max_age_roundtrip() {
		let mut map = HeaderMap::new();
		map.set::<AccessControlMaxAge>(&60u32);
		map.get::<AccessControlMaxAge>()
			.unwrap()
			.unwrap()
			.xpect_eq(60u32);
	}

	#[test]
	fn set_cookie_multi_value() {
		let mut map = HeaderMap::new();
		map.set_raw("set-cookie", "a=1");
		map.set_raw("set-cookie", "b=2");
		let cookies = map.get::<SetCookie>().unwrap().unwrap();
		cookies.len().xpect_eq(2);
		cookies[0].xpect_eq("a=1");
		cookies[1].xpect_eq("b=2");
	}

	#[test]
	fn cache_control_roundtrip() {
		let mut map = HeaderMap::new();
		map.set::<CacheControl>(
			&"no-cache, no-store, must-revalidate".to_string(),
		);
		map.get::<CacheControl>()
			.unwrap()
			.unwrap()
			.xpect_eq("no-cache, no-store, must-revalidate");
	}

	#[test]
	fn transfer_encoding_roundtrip() {
		let mut map = HeaderMap::new();
		map.set::<TransferEncoding>(&TransferEncodingValue::Chunked);
		map.get::<TransferEncoding>()
			.unwrap()
			.unwrap()
			.xpect_eq(TransferEncodingValue::Chunked);
	}

	#[test]
	fn transfer_encoding_variants() {
		for (raw, expected) in [
			("chunked", TransferEncodingValue::Chunked),
			("compress", TransferEncodingValue::Compress),
			("deflate", TransferEncodingValue::Deflate),
			("gzip", TransferEncodingValue::Gzip),
			("br", TransferEncodingValue::Other("br".to_string())),
		] {
			let mut map = HeaderMap::new();
			map.set_raw("transfer-encoding", raw);
			map.get::<TransferEncoding>()
				.unwrap()
				.unwrap()
				.xpect_eq(expected);
		}
	}
}
