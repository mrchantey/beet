use super::*;
use crate::prelude::*;
use http::HeaderMap;
use serde::Serialize;
use std::borrow::Cow;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

/// A cross-platform fetch function that works on both native and wasm targets.
/// While `reqwest` does work on wasm it still is a heavy build, instead cross-fetch
/// just uses fetch directly.
///
/// ## Targets
///
/// - Native: `reqwest`
/// - Wasm: `fetch`
#[derive(Debug, Clone)]
pub struct Request {
	pub url: Url,
	pub method: HttpMethod,
	pub headers: HeaderMap,
	pub timeout: Option<Duration>,
	pub body: Option<Vec<u8>>,
}

impl Request {
	/// Create a new request
	/// ## Panics
	/// Panics if the url cannot be parsed. This is a panic not error because
	/// the cases of invalid urls are tiny and weird.
	pub fn new(url: impl IntoUrl) -> Self {
		Self {
			url: url.into_url().unwrap(),
			method: HttpMethod::Get,
			headers: HeaderMap::new(),
			timeout: None,
			body: None,
		}
	}

	pub fn method(mut self, method: HttpMethod) -> Self {
		self.method = method;
		self
	}
	pub fn header(mut self, key: &str, value: &str) -> Result<Self> {
		self.headers.insert(
			http::header::HeaderName::from_str(key).map_err(|_| {
				Error::Serialization(format!("Invalid header name: {}", key))
			})?,
			http::header::HeaderValue::from_str(value).map_err(|_| {
				Error::Serialization(format!("Invalid header value: {}", value))
			})?,
		);
		Ok(self)
	}

	/// Shorthand for an `Authorization: Bearer <token>` header.
	pub fn auth_bearer(mut self, token: &str) -> Self {
		self.headers.insert(
			http::header::AUTHORIZATION,
			http::header::HeaderValue::from_str(&format!("Bearer {}", token))
				.unwrap(),
		);
		self
	}

	/// Serailizes the body to JSON and sets the `Content-Type` header to `application/json`.
	pub fn body<T: Serialize>(mut self, body: T) -> Result<Self> {
		self.body = Some(serde_json::to_vec(&body).map_err(|e| {
			Error::Serialization(format!(
				"Fail
			ed to serialize body: {}",
				e
			))
		})?);
		self.headers.insert(
			http::header::CONTENT_TYPE,
			http::header::HeaderValue::from_static("application/json"),
		);
		Ok(self)
	}

	/// insert a list of query parameters
	pub fn query<T1: Serialize, T2: Serialize>(
		mut self,
		query: &[(T1, T2)],
	) -> Result<Self> {
		{
			let mut pairs = self.url.query_pairs_mut();
			query
				.serialize(serde_urlencoded::Serializer::new(&mut pairs))
				.map_err(Error::serialization)?;
		}
		Ok(self)
	}

	/// Sets the body to a raw byte array and sets the `Content-Type` header to `application/octet-stream`.
	pub fn body_raw(mut self, body: Vec<u8>) -> Self {
		self.body = Some(body);
		self.headers.insert(
			http::header::CONTENT_TYPE,
			http::header::HeaderValue::from_static("application/octet-stream"),
		);
		self
	}
}

pub trait IntoUrl {
	fn into_url(self) -> Result<Url>;
}

impl IntoUrl for Url {
	fn into_url(self) -> Result<Url> {
		// With blob url the `self.has_host()` check is always false, so we
		// remove the `blob:` scheme and check again if the url is valid.
		#[cfg(target_arch = "wasm32")]
		if self.scheme() == "blob"
					&& self.path().starts_with("http") // Check if the path starts with http or https to avoid validating a `blob:blob:...` url.
					&& self.as_str()[5..].into_url().is_ok()
		{
			return Ok(self);
		}
		if self.has_host() {
			Ok(self)
		} else {
			Err(Error::Serialization(format!(
				"URL scheme is not allowed: {}",
				self
			)))
		}
	}
}
impl IntoUrl for &str {
	fn into_url(self) -> Result<Url> {
		Url::parse(self)
			.map_err(|_| Error::Serialization(format!("Invalid URL: {}", self)))
	}
}
impl IntoUrl for String {
	fn into_url(self) -> Result<Url> { self.as_str().into_url() }
}

impl IntoUrl for &Cow<'_, str> {
	fn into_url(self) -> Result<Url> { self.as_ref().into_url() }
}


#[cfg(test)]
mod test {
	use crate::cross_fetch::ResponseInner;
	use crate::prelude::*;
	use sweet_test::as_sweet::*;
	use sweet_utils::prelude::*;

	const HTTPBIN: &str = "https://httpbin.org";

	#[sweet_test::test]
	async fn works() {
		Request::new("https://example.com")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status_code())
			.xpect()
			.to_be(200);
	}

	#[sweet_test::test]
	async fn get_works() {
		Request::new(format!("{HTTPBIN}/get"))
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status_code())
			.xpect()
			.to_be(200);
	}

	#[sweet_test::test]
	async fn post_json_works() {
		Request::new(format!("{HTTPBIN}/post"))
			.method(HttpMethod::Post)
			.body(&serde_json::json!({"foo": "bar"}))
			.unwrap()
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status_code())
			.xpect()
			.to_be(200);
	}

	#[sweet_test::test]
	async fn custom_header_works() {
		Request::new(format!("{HTTPBIN}/headers"))
			.header("X-Foo", "Bar")
			.unwrap()
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status_code())
			.xpect()
			.to_be(200);
	}

	#[sweet_test::test]
	async fn put_and_delete_work() {
		Request::new(format!("{HTTPBIN}/put"))
			.method(HttpMethod::Put)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status_code())
			.xpect()
			.to_be(200);

		Request::new(format!("{HTTPBIN}/delete"))
			.method(HttpMethod::Delete)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status_code())
			.xpect()
			.to_be(200);
	}

	#[sweet_test::test]
	async fn body_raw_works() {
		Request::new(format!("{HTTPBIN}/post"))
			.method(HttpMethod::Post)
			.body_raw(b"rawbytes".to_vec())
			.send()
			.await
			.unwrap()
			.xmap(|res| res.text())
			.await
			.unwrap()
			.xpect()
			.to_contain("rawbytes");
	}


	#[test]
	fn query_params() {
		// #[derive(Serialize)]
		// struct Foo{
		Request::new(format!("{HTTPBIN}/get"))
			.query(&[("foo", (1, 2))])
			.xpect()
			.to_be_err();
	}

	#[sweet_test::test]
	async fn query_params_work() {
		Request::new(format!("{HTTPBIN}/get"))
			.query(&[("foo", "bar"), ("baz", "qux")])
			.unwrap()
			.send()
			.await
			.unwrap()
			.xmap(|res| res.text())
			.await
			.unwrap()
			.xpect()
			.to_contain("baz");
	}

	#[test]
	#[should_panic]
	fn bad_urp_fails() { Request::new("/foobar"); }
	#[test]
	fn invalid_header_fails() {
		Request::new("http://localhost")
			.header("bad\nheader", "val")
			.xpect()
			.to_be_err();
	}
}
