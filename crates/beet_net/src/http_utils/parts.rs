use crate::prelude::*;
use beet_core::prelude::*;
// use http::request;


type MultiMap = multimap::MultiMap<String, String, FixedHasher>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parts {
	/// The path split by `/` in the case of http,
	/// or positional arguments in the case of cli args.
	path: Vec<String>,
	/// The [`Self::path`] joined by `/`
	path_str: String,
	/// The query parameters or cli flags.
	/// ## Cli Flags
	///
	/// Short and long versions are untouched:
	/// `--foo -f` will be stored as `foo` and `f` seperately
	params: MultiMap,
	/// The http headers, can also be used to store environment variables
	/// for cli args.
	headers: MultiMap,
	/// The http version or cli command version, if applicable
	version: Option<String>,
}

/// Convert an http::HeaderMap to a MultiMap,
/// with all keys converted to lower kebab-case
fn header_map(map: &http::HeaderMap) -> MultiMap {
	use heck::ToKebabCase;
	let mut multi_map = MultiMap::with_hasher(FixedHasher::default());
	for (key, value) in map.iter() {
		let key = key.to_string().to_kebab_case();
		// header values can technically contain opaque bytes
		// but this is considered bad practice these days, we ignore them
		let value = value.to_str().unwrap_or("<opaque-bytes>").to_string();
		multi_map.insert(key.as_str().to_string(), value);
	}
	multi_map
}

impl Parts {
	pub fn path(&self) -> &Vec<String> { &self.path }
	pub fn version(&self) -> Option<&String> { self.version.as_ref() }
	pub fn params(&self) -> &MultiMap { &self.params }
	pub fn headers(&self) -> &MultiMap { &self.headers }

	pub fn get_param(&self, key: &str) -> Option<&String> {
		self.params.get_vec(key).and_then(|vals| vals.first())
	}

	pub fn get_params(&self, key: &str) -> Option<&Vec<String>> {
		self.params.get_vec(key)
	}

	pub fn get_header(&self, key: &str) -> Option<&String> {
		self.headers.get_vec(key).and_then(|vals| vals.first())
	}

	pub fn get_headers(&self, key: &str) -> Option<&Vec<String>> {
		self.headers.get_vec(key)
	}
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestParts {
	/// The method used to make the request
	method: HttpMethod,
	parts: Parts,
}

impl RequestParts {
	pub fn method(&self) -> &HttpMethod { &self.method }
}

impl std::ops::Deref for RequestParts {
	type Target = Parts;
	fn deref(&self) -> &Self::Target { &self.parts }
}


impl From<request::Parts> for RequestParts {
	fn from(parts: request::Parts) -> Self {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseParts {
	/// The status code of the response
	status: StatusCode,
	parts: Parts,
}

impl ResponseParts {
	pub fn status(&self) -> &StatusCode { &self.status }
}

impl std::ops::Deref for ResponseParts {
	type Target = Parts;
	fn deref(&self) -> &Self::Target { &self.parts }
}


impl From<http::response::Parts> for ResponseParts {
	fn from(parts: http::response::Parts) -> Self {}
}
