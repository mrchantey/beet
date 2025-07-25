use crate::prelude::*;
use bevy::ecs::system::RunSystemError;
use bevy::prelude::*;
use bytes::Bytes;
use http::StatusCode;
use http::response;
use std::convert::Infallible;

/// Added by the route or its layers, otherwise an empty [`StatusCode::Ok`]
/// will be returned.
#[derive(Debug, Clone, Resource)]
pub struct Response {
	pub parts: response::Parts,
	pub body: Option<Bytes>,
}

impl PartialEq for Response {
	fn eq(&self, other: &Self) -> bool {
		self.body == other.body
			&& self.parts.status == other.parts.status
			&& self.parts.headers == other.parts.headers
			&& self.parts.version == other.parts.version
		// && self.parts.extensions == other.parts.extensions
	}
}

impl Response {
	pub fn ok() -> Self { Self::from_status(StatusCode::OK) }
	pub fn not_found() -> Self {
		Self::from_status_body(StatusCode::NOT_FOUND, "Not Found")
	}
	pub fn status(&self) -> StatusCode { self.parts.status }
	pub fn from_status(status: StatusCode) -> Self {
		Self::from_parts(
			http::response::Builder::new()
				.status(status)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			None,
		)
	}

	pub fn from_status_body(
		status: StatusCode,
		body: impl AsRef<[u8]>,
	) -> Self {
		Self::from_parts(
			http::response::Builder::new()
				.status(status)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			Some(Bytes::copy_from_slice(body.as_ref())),
		)
	}


	pub fn from_parts(parts: response::Parts, body: Option<Bytes>) -> Self {
		Self { parts, body }
	}

	/// Create a response with the given body and content type.
	pub fn ok_body(body: impl AsRef<[u8]>, content_type: &str) -> Self {
		Self {
			parts: http::response::Builder::new()
				.status(StatusCode::OK)
				.header(http::header::CONTENT_TYPE, content_type)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			body: Some(Bytes::copy_from_slice(body.as_ref())),
		}
	}

	pub fn body_str(self) -> Result<String> {
		self.body
			.map(|b| String::from_utf8(b.to_vec()).unwrap_or_default())
			.ok_or_else(|| bevyhow!("Response body is empty"))
	}

	pub fn into_http(self) -> http::Response<Bytes> {
		http::Response::from_parts(
			self.parts,
			self.body.unwrap_or_else(|| Bytes::new()),
		)
	}

	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub fn into_axum(self) -> axum::response::Response {
		axum::response::Response::from_parts(
			self.parts,
			self.body.map_or_else(
				|| axum::body::Body::empty(),
				|bytes| axum::body::Body::from(bytes),
			),
		)
	}

	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub async fn from_axum(resp: axum::response::Response) -> Self {
		let (parts, body) = resp.into_parts();
		let bytes = axum::body::to_bytes(body, usize::MAX).await.ok();
		Self { parts, body: bytes }
	}
}


// not specific enough
// impl Into<Response> for () {
// 	fn into(self) -> Response { Response::ok() }
// }


impl Into<Response> for BevyError {
	fn into(self) -> Response { HttpError::from_opaque(self).into() }
}

impl Into<Response> for RunSystemError {
	fn into(self) -> Response { HttpError::from_opaque(self).into() }
}

impl IntoResponse for Bytes {
	fn into_response(self) -> Response {
		Response::ok_body(&self, "application/octet-stream")
	}
}
impl IntoResponse for &[u8] {
	fn into_response(self) -> Response {
		Response::ok_body("dsds", "application/octet-stream")
	}
}

impl Into<http::Response<Bytes>> for Response {
	fn into(self) -> http::Response<Bytes> { self.into_http() }
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
impl Into<axum::response::Response> for Response {
	fn into(self) -> axum::response::Response { self.into_axum() }
}


/// Allows for blanket implementation of `Into<Response>`,
/// including `Result<T,E>` where `T` and `E` both implement `IntoResponse`
/// and  Option<T> where `T` implements `IntoResponse`, and [`None`] is not found.
pub trait IntoResponse {
	fn into_response(self) -> Response;
}

impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
	fn into_response(self) -> Response {
		match self {
			Ok(t) => t.into_response(),
			Err(e) => e.into_response(),
		}
	}
}

impl IntoResponse for Infallible {
	fn into_response(self) -> Response {
		unreachable!("Infallible cannot be converted to a response");
	}
}

impl<T: TryInto<Response>> IntoResponse for T
where
	T::Error: IntoResponse,
{
	fn into_response(self) -> Response {
		match self.try_into() {
			Ok(response) => response,
			Err(err) => err.into_response(),
		}
	}
}

/// None = not found, matching http principles ie crud operations
impl<T: IntoResponse> IntoResponse for Option<T> {
	fn into_response(self) -> Response {
		match self {
			Some(t) => t.into_response(),
			None => {
				Response::from_status_body(StatusCode::NOT_FOUND, b"Not Found")
			}
		}
	}
}

// impl Default for Response {
// 	fn default() -> Self {
// 		Self {
// 			// one does not simply Parts::default()
// 			parts: http::response::Builder::new()
// 				.body(())
// 				.unwrap()
// 				.into_parts()
// 				.0,
// 			body: None,
// 		}
// 	}
// }
