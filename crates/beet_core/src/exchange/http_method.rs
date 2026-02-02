//! HTTP method types for request handling.
//!
//! This module provides [`HttpMethod`], a high-level enum for HTTP methods,
//! and [`MethodFilter`] for specifying which methods a route should respond to.
//!
//! # Why Not `http::Method`?
//!
//! The standard [`http::Method`] type is a low-level representation that is
//! case-sensitive and doesn't implement useful traits like `Copy` or `serde`.
//! This module provides a safer, more ergonomic alternative.

use crate::prelude::*;
use std::fmt;

/// Caching strategy for route responses.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum CacheStrategy {
	/// An endpoint that may produce different responses for the same path and method,
	/// and should not be cached.
	#[default]
	Dynamic,
	/// An endpoint that always returns the same response for a given
	/// path and method, making it suitable for SSG and caching.
	Static,
}


/// A high-level representation of HTTP methods.
///
/// This enum provides a safer alternative to [`http::Method`] with:
/// - Case-insensitive parsing
/// - `Copy` semantics
/// - Serde support
/// - Rusty naming conventions (`Get` instead of `GET`)
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// let method: HttpMethod = "POST".parse().unwrap();
/// assert_eq!(method, HttpMethod::Post);
/// assert!(method.has_body());
/// ```
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Copy,
	Component,
	Reflect,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum HttpMethod {
	/// GET request for retrieving resources.
	#[default]
	Get,
	/// POST request for creating resources.
	Post,
	/// PUT request for replacing resources.
	Put,
	/// PATCH request for partial updates.
	Patch,
	/// DELETE request for removing resources.
	Delete,
	/// OPTIONS request for CORS preflight.
	Options,
	/// HEAD request for metadata only.
	Head,
	/// TRACE request for debugging.
	Trace,
	/// CONNECT request for tunneling.
	Connect,
}

impl HttpMethod {
	#[allow(unused)]
	const METHODS: [&str; 9] = [
		"get", "post", "put", "delete", "head", "options", "connect", "trace",
		"patch",
	];

	/// All HTTP methods.
	pub const ALL: [HttpMethod; 9] = [
		HttpMethod::Get,
		HttpMethod::Post,
		HttpMethod::Put,
		HttpMethod::Delete,
		HttpMethod::Head,
		HttpMethod::Options,
		HttpMethod::Connect,
		HttpMethod::Trace,
		HttpMethod::Patch,
	];

	/// Returns whether this method typically has a request body.
	///
	/// Returns `true` for `POST`, `PUT`, and `PATCH`.
	pub fn has_body(&self) -> bool {
		matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
	}

	/// Returns the method name in lowercase.
	pub fn to_string_lowercase(&self) -> String {
		self.to_string().to_ascii_lowercase()
	}

	/// Converts to the [`http::Method`] type.
	#[cfg(feature = "http")]
	pub fn into_http(self) -> http::Method { self.into() }
}

#[cfg(feature = "http")]
impl From<http::Method> for HttpMethod {
	fn from(value: http::Method) -> Self { Self::from(&value) }
}

#[cfg(feature = "http")]
impl From<&http::Method> for HttpMethod {
	fn from(method: &http::Method) -> Self {
		// case insensitive
		match method.as_str().to_ascii_uppercase().as_str() {
			"GET" => HttpMethod::Get,
			"POST" => HttpMethod::Post,
			"PUT" => HttpMethod::Put,
			"PATCH" => HttpMethod::Patch,
			"DELETE" => HttpMethod::Delete,
			"OPTIONS" => HttpMethod::Options,
			"HEAD" => HttpMethod::Head,
			"TRACE" => HttpMethod::Trace,
			"CONNECT" => HttpMethod::Connect,
			_ => panic!("Unknown HTTP method: {}", method),
		}
	}
}

#[cfg(feature = "http")]
impl Into<http::Method> for HttpMethod {
	fn into(self) -> http::Method {
		match self {
			HttpMethod::Get => http::Method::GET,
			HttpMethod::Post => http::Method::POST,
			HttpMethod::Put => http::Method::PUT,
			HttpMethod::Patch => http::Method::PATCH,
			HttpMethod::Delete => http::Method::DELETE,
			HttpMethod::Options => http::Method::OPTIONS,
			HttpMethod::Head => http::Method::HEAD,
			HttpMethod::Trace => http::Method::TRACE,
			HttpMethod::Connect => http::Method::CONNECT,
		}
	}
}


impl fmt::Display for HttpMethod {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", match self {
			HttpMethod::Get => "Get",
			HttpMethod::Post => "Post",
			HttpMethod::Put => "Put",
			HttpMethod::Patch => "Patch",
			HttpMethod::Delete => "Delete",
			HttpMethod::Options => "Options",
			HttpMethod::Head => "Head",
			HttpMethod::Trace => "Trace",
			HttpMethod::Connect => "Connect",
		})
	}
}

impl std::str::FromStr for HttpMethod {
	type Err = BevyError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		// case insensitive
		Ok(match s.to_ascii_uppercase().as_str() {
			"GET" => HttpMethod::Get,
			"POST" => HttpMethod::Post,
			"PUT" => HttpMethod::Put,
			"PATCH" => HttpMethod::Patch,
			"DELETE" => HttpMethod::Delete,
			"OPTIONS" => HttpMethod::Options,
			"HEAD" => HttpMethod::Head,
			"TRACE" => HttpMethod::Trace,
			"CONNECT" => HttpMethod::Connect,
			_ => bevybail!("Unknown HTTP method: {}", s),
		})
	}
}


/// Specifies which HTTP methods a route should respond to.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// // Single method
/// let filter: MethodFilter = HttpMethod::Get.into();
///
/// // Multiple methods
/// let methods = vec![HttpMethod::Get, HttpMethod::Post];
/// let filter = MethodFilter::new(methods);
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Component, Reflect, Deref, DerefMut,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct MethodFilter {
	methods: Vec<HttpMethod>,
}

impl Default for MethodFilter {
	fn default() -> Self { HttpMethod::Get.into() }
}

impl Into<MethodFilter> for HttpMethod {
	fn into(self) -> MethodFilter { MethodFilter::new(vec![self]) }
}

impl Into<MethodFilter> for Vec<HttpMethod> {
	fn into(self) -> MethodFilter { MethodFilter::new(self) }
}

impl MethodFilter {
	/// Creates a new [`MethodFilter`] with the given methods.
	pub fn new(methods: Vec<HttpMethod>) -> Self { Self { methods } }

	/// Merges another filter into this one, deduplicating methods.
	pub fn merge(mut self, other: MethodFilter) -> MethodFilter {
		self.extend(other.methods);
		self.sort();
		self.dedup();
		self
	}
}
