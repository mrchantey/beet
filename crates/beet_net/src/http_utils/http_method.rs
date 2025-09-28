use beet_core::prelude::*;
use std::fmt;


#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum CacheStrategy {
	/// An endpoint that may produce different responses for the same path and method,
	/// and should not be cached
	#[default]
	Dynamic,
	/// An endpoint that always returns the same response for a given
	/// path and method, making it suitable for ssg and caching.
	Static,
}


/// Alternative to the [`http::Method`] which is a low level representation of HTTP methods
/// and quite error prone in this high level context. For example
/// `http::method::from_str("get") != http::Method::GET` due to
/// case sensitivity.
/// Instead we use an enum which is safer and allows implementing `Copy`, `serde`, etc.
///
/// Additionally the naming convention follows Rusty conventions rather
/// than HTTP conventions, ie `Get` instead of `GET`.
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
	#[default]
	Get,
	Post,
	Put,
	Patch,
	Delete,
	Options,
	Head,
	Trace,
	Connect,
}

impl HttpMethod {
	#[allow(unused)]
	const METHODS: [&str; 9] = [
		"get", "post", "put", "delete", "head", "options", "connect", "trace",
		"patch",
	];
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

	/// Whether this method is one of the HTTP methods that typically
	/// has a request body, such as `POST`, `PUT`, or `PATCH`.
	pub fn has_body(&self) -> bool {
		matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
	}
	pub fn to_string_lowercase(&self) -> String {
		self.to_string().to_ascii_lowercase()
	}
	pub fn into_http(self) -> http::Method { self.into() }
}
impl From<http::Method> for HttpMethod {
	fn from(value: http::Method) -> Self { Self::from(&value) }
}
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


/// Specify which HTTP methods a route should respond to
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
	pub fn new(methods: Vec<HttpMethod>) -> Self { methods.into() }

	pub fn merge(mut self, other: MethodFilter) -> MethodFilter {
		self.extend(other.methods);
		self.sort();
		self.dedup();
		self
	}
}
