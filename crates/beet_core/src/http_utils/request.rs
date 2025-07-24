use crate::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use http::request;

/// A generalized request [`Resource`] added to every route app before the
/// request is processed.
#[derive(Debug, Clone, Resource)]
pub struct Request {
	pub parts: request::Parts,
	pub body: Option<Bytes>,
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
	) -> Result<Self, axum::extract::rejection::BytesRejection> {
		use axum::extract::FromRequest;
		let (parts, body) = request.into_parts();
		let bytes = if HttpExt::has_body(&parts) {
			let request =
				axum::extract::Request::from_parts(parts.clone(), body);
			let bytes = Bytes::from_request(request, state).await?;
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

impl<T: Into<Bytes>> From<http::Request<T>> for Request {
	fn from(request: http::Request<T>) -> Self { Self::from_http(request) }
}
