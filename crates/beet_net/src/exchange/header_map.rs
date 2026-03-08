//! Typed header map for HTTP-style headers.
//!
//! [`HeaderMap`] wraps a [`MultiMap`] with kebab-case key normalization,
//! and provides typed access via the [`Header`] trait.
//!
//! For concrete header types see [`super::header`].
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use beet_net::headers;
//! let mut map = HeaderMap::new();
//! map.set::<headers::ContentType>(MediaType::Json);
//! let media_type = map.get::<headers::ContentType>().unwrap().unwrap();
//! assert_eq!(media_type, MediaType::Json);
//! ```

use super::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// A multimap of HTTP-style headers with kebab-case key normalization.
///
/// All keys are normalized to lowercase with underscores replaced by hyphens
/// on insertion and lookup, ensuring case-insensitive, canonical access.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeaderMap(MultiMap<String, String>);

/// Normalize a header key to lowercase kebab-case.
///
/// Converts uppercase to lowercase and replaces `_` with `-`.
/// Returns [`Cow::Borrowed`] if no transformation is needed.
pub fn to_kebab_case(val: &str) -> Cow<'_, str> {
	let needs_transform = val
		.bytes()
		.any(|byte| byte.is_ascii_uppercase() || byte == b'_');
	if needs_transform {
		let transformed: String = val
			.bytes()
			.map(|byte| {
				if byte == b'_' {
					b'-'
				} else {
					byte.to_ascii_lowercase()
				}
			})
			.map(|byte| byte as char)
			.collect();
		Cow::Owned(transformed)
	} else {
		Cow::Borrowed(val)
	}
}

impl HeaderMap {
	/// Create a new empty header map.
	pub fn new() -> Self { Self(MultiMap::new()) }

	/// Insert a raw header value. The key is normalized to kebab-case.
	pub fn set_raw(&mut self, key: impl AsRef<str>, value: impl Into<String>) {
		let key = to_kebab_case(key.as_ref()).into_owned();
		self.0.insert(key, value.into());
	}

	/// Get the first raw string value for a header key.
	pub fn first_raw(&self, key: &str) -> Option<&str> {
		let key = to_kebab_case(key);
		self.0
			.get_vec(key.as_ref())
			.and_then(|vals| vals.first().map(|val| val.as_str()))
	}

	/// Get all raw string values for a header key.
	pub fn get_raw(&self, key: &str) -> Option<&Vec<String>> {
		let key = to_kebab_case(key);
		self.0.get_vec(key.as_ref())
	}

	/// Get a typed header value. Returns `None` if the header is absent,
	/// or `Some(Err(..))` if parsing fails.
	pub fn get<H: Header>(&self) -> Option<Result<H::Value>> {
		self.get_raw(H::KEY).map(|vals| H::parse(vals))
	}

	/// Set a typed header, replacing any existing values for that key.
	pub fn set<H: Header>(&mut self, value: impl Into<H::Value>) {
		let key = to_kebab_case(H::KEY).into_owned();
		self.0.remove(&key);
		for val in H::serialize(value.into()) {
			self.0.insert(key.clone(), val);
		}
	}

	/// Set the `Content-Type` header from a [`MediaType`].
	pub fn set_content_type(&mut self, media_type: MediaType) {
		self.set::<header::ContentType>(media_type);
	}

	/// Check if a header key exists.
	pub fn contains_key(&self, key: &str) -> bool {
		let key = to_kebab_case(key);
		self.0.contains_key(key.as_ref())
	}

	/// Returns true if the map contains no headers.
	pub fn is_empty(&self) -> bool { self.0.is_empty() }

	/// Returns the number of distinct header keys.
	pub fn len(&self) -> usize { self.0.len() }

	/// Iterate over all key-value pairs.
	pub fn iter_all(&self) -> impl Iterator<Item = (&String, &Vec<String>)> {
		self.0.iter_all()
	}

	/// Iterate over all keys.
	pub fn keys(&self) -> impl Iterator<Item = &String> { self.0.keys() }

	/// Remove a header key and all its values.
	pub fn remove(&mut self, key: &str) -> Option<Vec<String>> {
		let key = to_kebab_case(key);
		self.0.remove(key.as_ref())
	}

	/// Clear all headers.
	pub fn clear(&mut self) { self.0.clear(); }

	/// Returns a reference to the inner multimap.
	pub fn inner(&self) -> &MultiMap<String, String> { &self.0 }

	/// Returns a mutable reference to the inner multimap.
	///
	/// Use with caution — keys inserted directly will not be normalized.
	pub fn inner_mut(&mut self) -> &mut MultiMap<String, String> { &mut self.0 }
}

/// Convert a raw [`MultiMap`] into a [`HeaderMap`], normalizing all keys.
impl From<MultiMap<String, String>> for HeaderMap {
	fn from(raw: MultiMap<String, String>) -> Self {
		let mut map = HeaderMap::new();
		for (key, values) in raw.iter_all() {
			let normalized = to_kebab_case(key).into_owned();
			for value in values {
				map.0.insert(normalized.clone(), value.clone());
			}
		}
		map
	}
}

// ============================================================================
// Header trait
// ============================================================================

/// A typed header that can be parsed from raw string values.
///
/// Implement this trait for zero-sized marker types to provide typed access
/// to specific headers via [`HeaderMap::get`].
pub trait Header {
	/// The parsed value type for this header.
	type Value;
	/// The canonical lowercase kebab-case key, ie `"content-type"`.
	const KEY: &'static str;
	/// Parse the header from its raw string values.
	fn parse(values: &Vec<String>) -> Result<Self::Value>;
	/// Serialize the typed value into raw header strings.
	fn serialize(value: Self::Value) -> Vec<String>;
}

#[cfg(test)]
mod test {
	use super::header as headers;
	use super::*;

	#[test]
	fn to_kebab_case_lowercase() {
		to_kebab_case("content-type").xpect_eq("content-type");
	}

	#[test]
	fn to_kebab_case_uppercase() {
		to_kebab_case("Content-Type").xpect_eq("content-type");
	}

	#[test]
	fn to_kebab_case_underscores() {
		to_kebab_case("content_type").xpect_eq("content-type");
	}

	#[test]
	fn to_kebab_case_mixed() {
		to_kebab_case("X_Custom_Header").xpect_eq("x-custom-header");
	}

	#[test]
	fn to_kebab_case_borrows_when_already_normalized() {
		let result = to_kebab_case("content-type");
		matches!(result, Cow::Borrowed(_)).xpect_true();
	}

	#[test]
	fn insert_and_get_str() {
		let mut headers = HeaderMap::new();
		headers.set::<headers::ContentType>(MediaType::Json);
		headers
			.get::<headers::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn case_insensitive_lookup() {
		let mut headers = HeaderMap::new();
		headers.set::<headers::ContentType>(MediaType::Html);
		// All casings resolve to the same normalized key
		headers
			.get::<headers::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Html);
		headers
			.first_raw("Content-Type")
			.unwrap()
			.xpect_eq("text/html");
		headers
			.first_raw("CONTENT-TYPE")
			.unwrap()
			.xpect_eq("text/html");
		headers
			.first_raw("content_type")
			.unwrap()
			.xpect_eq("text/html");
	}

	#[test]
	fn multiple_values() {
		let mut headers = HeaderMap::new();
		headers.set_raw("set-cookie", "a=1");
		headers.set_raw("set-cookie", "b=2");
		let cookies = headers.get::<headers::SetCookie>().unwrap().unwrap();
		cookies.len().xpect_eq(2);
		cookies[0].as_str().xpect_eq("a=1");
		cookies[1].as_str().xpect_eq("b=2");
	}

	#[test]
	fn contains_key_normalized() {
		let mut headers = HeaderMap::new();
		headers.set_raw("x-custom", "value");
		headers.contains_key("x-custom").xpect_true();
		headers.contains_key("X_Custom").xpect_true();
		headers.contains_key("x-missing").xpect_false();
	}

	#[test]
	fn remove_header() {
		let mut headers = HeaderMap::new();
		headers.set_raw("x-custom", "value");
		headers.remove("x-custom").unwrap().len().xpect_eq(1);
		headers.contains_key("x-custom").xpect_false();
	}

	#[test]
	fn set_content_type_helper() {
		let mut headers = HeaderMap::new();
		headers.set_content_type(MediaType::Json);
		headers
			.get::<headers::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
	}

	#[test]
	fn from_multimap() {
		let mut raw = MultiMap::new();
		raw.insert("Content_Type".to_string(), "text/html".to_string());
		let headers = HeaderMap::from(raw);
		headers
			.get::<headers::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Html);
	}

	#[test]
	fn header_map_empty() {
		let headers = HeaderMap::new();
		headers.is_empty().xpect_true();
		headers.len().xpect_eq(0);
	}

	#[test]
	fn header_map_len() {
		let mut headers = HeaderMap::new();
		headers.set_raw("a", "1");
		headers.set_raw("b", "2");
		headers.len().xpect_eq(2);
		// Same key, different values — still one key
		headers.set_raw("a", "3");
		headers.len().xpect_eq(2);
	}
}
