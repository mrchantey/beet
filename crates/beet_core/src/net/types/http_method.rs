#[cfg(feature = "tokens")]
use crate::as_beet::*;
use std::fmt;

/// Alternative to the [`http::Method`] which is a low level representation of HTTP methods
/// and quite error prone in this high level context. For example
/// `http::method::from_str("get") != http::Method::GET` due to
/// case sensitivity.
/// Instead we use an enum which is safer and allows implementing `Copy`, `serde`, etc.
///
/// Additionally the naming convention follows Rusty conventions rather
/// than HTTP conventions, ie `Get` instead of `GET`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Copy)]
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
	pub const METHODS: [&str; 9] = [
		"get", "post", "put", "delete", "head", "options", "connect", "trace",
		"patch",
	];

	/// Whether this method is one of the HTTP methods that typically
	/// has a request body, such as `POST`, `PUT`, or `PATCH`.
	pub fn has_body(&self) -> bool {
		matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
	}
	pub fn to_string_lowercase(&self) -> String {
		self.to_string().to_ascii_lowercase()
	}
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
	type Err = anyhow::Error;
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
			_ => anyhow::bail!("Unknown HTTP method: {}", s),
		})
	}
}
