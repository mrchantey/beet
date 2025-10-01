use std::str::FromStr;

use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use http::Uri;
use http::header::IntoHeaderName;
use http::request;

/// A generalized request [`Resource`] added to every route app before the
/// request is processed.
#[derive(Debug, Clone, Resource)]
pub struct Request {
	pub parts: request::Parts,

	pub body: Option<Bytes>,
}

impl Request {
	pub fn new(method: HttpMethod, path: impl AsRef<str>) -> Self {
		let parts = request::Builder::new()
			.method(method)
			.uri(path.as_ref())
			.body(())
			.expect("Failed to create request parts")
			.into_parts()
			.0;
		Self { parts, body: None }
	}


	pub fn from_parts(parts: request::Parts, body: Option<Bytes>) -> Self {
		Self { parts, body }
	}

	pub fn get(path: impl AsRef<str>) -> Self {
		Self {
			parts: request::Builder::new()
				.method(http::Method::GET)
				.uri(path.as_ref())
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: None,
		}
	}
	pub fn post(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Post)
	}
	pub fn put(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Put)
	}
	pub fn delete(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Delete)
	}
	pub fn patch(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Patch)
	}
	pub fn head(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Head)
	}
	pub fn options(path: impl AsRef<str>) -> Self {
		Self::get(path).with_method(HttpMethod::Options)
	}

	pub fn with_method(mut self, method: HttpMethod) -> Self {
		self.parts.method = method.into();
		self
	}

	pub fn with_body(mut self, body: impl AsRef<[u8]>) -> Self {
		self.body = Some(Bytes::copy_from_slice(body.as_ref()));
		self
	}

	/// Add a json body, and set the content type to application/json
	#[cfg(feature = "serde")]
	pub fn with_json_body<T: serde::Serialize>(
		self,
		body: &T,
	) -> Result<Self, serde_json::Error> {
		use beet_core::prelude::*;

		let body = serde_json::to_string(body)?;
		self.with_body(body)
			.with_content_type("application/json")
			.xok()
	}

	pub fn set_body(&mut self, body: impl AsRef<[u8]>) -> &mut Self {
		self.body = Some(Bytes::copy_from_slice(body.as_ref()));
		self
	}
	pub fn with_header<K: IntoHeaderName>(
		mut self,
		key: K,
		value: &str,
	) -> Self {
		self.parts
			.headers
			.insert(key, http::header::HeaderValue::from_str(value).unwrap());
		self
	}

	/// Shorthand for an `Authorization: Bearer <token>` header.
	pub fn with_auth_bearer(mut self, token: &str) -> Self {
		self.parts.headers.insert(
			http::header::AUTHORIZATION,
			http::header::HeaderValue::from_str(&format!("Bearer {}", token))
				.unwrap(),
		);
		self
	}

	/// Set the content type header.
	pub fn with_content_type(mut self, content_type: &str) -> Self {
		self.parts.headers.insert(
			http::header::CONTENT_TYPE,
			http::header::HeaderValue::from_str(content_type).unwrap(),
		);
		self
	}

	/// Parse both the key and value as valid URL query parameters.
	#[cfg(feature = "serde")]
	pub fn parse_query_param<
		T1: serde::Serialize + ?Sized,
		T2: serde::Serialize,
	>(
		self,
		key: &T1,
		value: &T2,
	) -> Result<Self> {
		let key = serde_urlencoded::to_string(key)?;
		let value = serde_urlencoded::to_string(value)?;
		self.with_query_param(&key, &value)
	}

	/// Insert a query parameter into the request URI without checking it is
	/// a valid URL, for the checked version use [`Self::parse_query_param`].
	pub fn with_query_param(self, key: &str, value: &str) -> Result<Self> {
		let updated_query = if let Some(query) = self.parts.uri.query() {
			format!("{}&{}={}", query, key, value)
		} else {
			format!("{}={}", key, value)
		};
		self.with_query_string(&updated_query)
	}
	pub fn with_query_string(mut self, query: &str) -> Result<Self> {
		let path = self.parts.uri.path();
		let path_and_query = format!("{}?{}", path, query);
		let mut uri_parts = self.parts.uri.clone().into_parts();
		uri_parts.path_and_query =
			Some(http::uri::PathAndQuery::from_str(&path_and_query)?);
		self.parts.uri = Uri::from_parts(uri_parts)?;
		Ok(self)
	}

	pub fn method(&self) -> HttpMethod {
		HttpMethod::from(self.parts.method.clone())
	}

	pub fn body_str(&self) -> Option<String> {
		self.body
			.as_ref()
			.map(|b| String::from_utf8(b.to_vec()).unwrap_or_default())
	}

	pub fn from_http<T: Into<Bytes>>(request: http::Request<T>) -> Self {
		let (parts, body) = request.into_parts();
		let bytes = if HttpExt::has_body(&parts) {
			Some(Bytes::from(body.into()))
		} else {
			None
		};
		Self { parts, body: bytes }
	}

	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub async fn from_axum<S: 'static + Send + Sync>(
		request: axum::extract::Request,
		state: &S,
	) -> HttpResult<Self> {
		use axum::extract::FromRequest;
		let (parts, body) = request.into_parts();
		let bytes = if HttpExt::has_body(&parts) {
			let request =
				axum::extract::Request::from_parts(parts.clone(), body);
			let bytes =
				Bytes::from_request(request, state).await.map_err(|err| {
					HttpError::bad_request(format!(
						"Failed to extract request: {}",
						err
					))
				})?;
			Some(bytes)
		} else {
			None
		};
		Ok(Self { parts, body: bytes })
	}
	pub fn into_http_request(self) -> http::Request<Bytes> { self.into() }
}


impl From<RouteInfo> for Request {
	fn from(route_info: RouteInfo) -> Self {
		let method: http::Method = route_info.method.into();
		Self {
			parts: request::Builder::new()
				.method(method)
				.uri(route_info.path.to_string_lossy().as_ref())
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: None,
		}
	}
}

impl Into<()> for Request {
	fn into(self) -> () {}
}

impl Into<http::Request<Bytes>> for Request {
	fn into(self) -> http::Request<Bytes> {
		http::Request::from_parts(
			self.parts,
			self.body.unwrap_or_else(Bytes::new),
		)
	}
}


impl From<&str> for Request {
	fn from(path: &str) -> Self { Request::get(path) }
}

/// Blanket impl for any type that is `TryFrom<Request, Error:IntoResponse>`.
pub trait FromRequest<M>: Sized {
	fn from_request(request: Request) -> Result<Self, Response>;
}

impl<T, E> FromRequest<E> for T
where
	T: TryFrom<Request, Error = E>,
	E: IntoResponse,
{
	fn from_request(request: Request) -> Result<Self, Response> {
		request.try_into().map_err(|e: E| e.into_response())
	}
}


impl<T: Into<Bytes>> From<http::Request<T>> for Request {
	fn from(request: http::Request<T>) -> Self { Self::from_http(request) }
}
