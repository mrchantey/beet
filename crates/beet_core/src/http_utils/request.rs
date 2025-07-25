use std::str::FromStr;

use crate::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use http::Uri;
use http::request;

/// A generalized request [`Resource`] added to every route app before the
/// request is processed.
#[derive(Debug, Clone, Resource)]
pub struct Request {
	pub parts: request::Parts,

	pub body: Option<Bytes>,
}

impl Request {
	pub fn new(parts: request::Parts, body: Option<Bytes>) -> Self {
		Self { parts, body }
	}

	pub fn get(path: &str) -> Self {
		Self {
			parts: request::Builder::new()
				.method(http::Method::GET)
				.uri(path)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: None,
		}
	}

	pub fn set_body<T: Into<Bytes>>(&mut self, body: T) -> &mut Self {
		self.body = Some(body.into());
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

	/// Parse both the key and value as valid URL query parameters.
	#[cfg(feature = "serde")]
	pub fn parse_query_param<T1: serde::Serialize, T2: serde::Serialize>(
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
	pub fn with_query_param(mut self, key: &str, value: &str) -> Result<Self> {
		let path = self.parts.uri.path();
		let query = self.parts.uri.query();
		let path_and_query = if let Some(query) = query {
			format!("{}?{}&{}={}", path, query, key, value)
		} else {
			format!("{}?{}={}", path, key, value)
		};
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

	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
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
