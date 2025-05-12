use std::fmt;


/// Alternative to the [`http::Method`] which is a low level representation of HTTP methods
/// and quite error prone in this high level context. For example
/// `http::method::from_str("get") != http::Method::GET` due to
/// case sensitivity.
/// Instead we use an enum which is far easier to use, including
/// being `serde` and `Copy`.
///
/// Additionally the naming convention follows Rusty conventions rather
/// than HTTP conventions, ie `Get` instead of `GET`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
	pub fn has_body(&self) -> bool {
		matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
	}
}

impl From<http::Method> for HttpMethod {
	fn from(method: http::Method) -> Self {
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
