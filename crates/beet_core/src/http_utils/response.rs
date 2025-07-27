use crate::prelude::*;
use beet_utils::utils::PipelineTarget;
use bevy::ecs::system::RunSystemError;
use bevy::prelude::*;
use bytes::Bytes;
use http::StatusCode;
use http::header::CONTENT_TYPE;
use http::response;
use std::convert::Infallible;

/// Added by the route or its layers, otherwise an empty [`StatusCode::Ok`]
/// will be returned.
#[derive(Debug, Resource)]
pub struct Response {
	pub parts: response::Parts,
	pub body: Body,
}

pub enum Body {
	Bytes(Bytes),
	// TODO
	Stream,
}

impl Into<Body> for Bytes {
	fn into(self) -> Body { Body::Bytes(self) }
}

impl std::fmt::Debug for Body {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Body::Bytes(bytes) => write!(f, "Body::Bytes({:?})", bytes),
			Body::Stream => write!(f, "Body::Stream(...)"),
		}
	}
}

impl Body {
	pub async fn into_bytes(self) -> Result<Bytes> {
		match self {
			Body::Bytes(bytes) => Ok(bytes),
			Body::Stream => {
				todo!()
			}
		}
	}
	pub fn bytes_eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Body::Bytes(a), Body::Bytes(b)) => a == b,
			_ => false,
		}
	}
}

impl PartialEq for Response {
	fn eq(&self, other: &Self) -> bool {
		self.body.bytes_eq(&other.body)
			&& self.parts.status == other.parts.status
			&& self.parts.headers == other.parts.headers
			&& self.parts.version == other.parts.version
		// && self.parts.extensions == other.parts.extensions
	}
}

impl Response {
	pub fn ok() -> Self { Self::from_status(StatusCode::OK) }
	pub fn not_found() -> Self { Self::from_status(StatusCode::NOT_FOUND) }
	pub fn status(&self) -> StatusCode { self.parts.status }
	pub fn from_status(status: StatusCode) -> Self {
		Self::from_parts(
			http::response::Builder::new()
				.status(status)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			Default::default(),
		)
	}

	pub fn from_status_body(
		status: StatusCode,
		body: impl AsRef<[u8]>,
		content_type: &str,
	) -> Self {
		Self::from_parts(
			http::response::Builder::new()
				.status(status)
				.header(http::header::CONTENT_TYPE, content_type)
				.body(())
				.unwrap()
				.into_parts()
				.0,
			Bytes::copy_from_slice(body.as_ref()),
		)
	}

	pub fn header_matches(
		&self,
		header: http::header::HeaderName,
		value: &str,
	) -> bool {
		self.parts
			.headers
			.get(&header)
			.map_or(false, |v| v == value)
	}

	pub fn from_parts(parts: response::Parts, body: Bytes) -> Self {
		Self {
			parts,
			body: body.into(),
		}
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
			body: Bytes::copy_from_slice(body.as_ref()).into(),
		}
	}

	pub async fn text(self) -> Result<String> {
		let bytes = self.body.into_bytes().await?;
		String::from_utf8(bytes.to_vec())?.xok()
	}

	#[cfg(feature = "serde")]
	pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
		let body = self.body.into_bytes().await?;
		serde_json::from_slice::<T>(&body).map_err(|e| {
			bevyhow!("Failed to deserialize response body\n {}", e)
		})
	}

	pub async fn into_http(self) -> Result<http::Response<Bytes>> {
		let bytes = self.body.into_bytes().await?;
		http::Response::from_parts(self.parts, bytes).xok()
	}

	/// Convert the response into a result,
	/// returning an error if the status code is not successful 2xx.
	pub async fn into_result(self) -> Result<Self, HttpError> {
		if self.parts.status.is_success() {
			Ok(self)
		} else {
			Err(self.into_error().await)
		}
	}

	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub async fn into_axum(self) -> axum::response::Response {
		use axum::response::IntoResponse;

		match self.body.into_bytes().await {
			Ok(bytes) => axum::response::Response::from_parts(
				self.parts,
				axum::body::Body::from(bytes),
			),
			Err(_) => {
				(StatusCode::INTERNAL_SERVER_ERROR, "failed to read body")
					.into_response()
			}
		}
	}

	#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
	pub async fn from_axum(resp: axum::response::Response) -> Result<Self> {
		let (parts, body) = resp.into_parts();
		let body = axum::body::to_bytes(body, usize::MAX).await?;
		Self {
			parts,
			body: body.into(),
		}
		.xok()
	}
	/// convert the response into an error even if it is a 2xx status code,
	/// extracting the status code and message from the body.
	/// For a method that checks the status code see [`Response::into_result`].
	pub async fn into_error(self) -> HttpError {
		let is_text = self.header_matches(CONTENT_TYPE, "text/plain");
		let status = self.status();
		let Ok(bytes) = self.body.into_bytes().await else {
			return HttpError::internal_error("Failed to read response body");
		};
		let message = if is_text && !bytes.is_empty() {
			String::from_utf8_lossy(&bytes).to_string()
		} else {
			Default::default()
		};

		HttpError {
			status_code: status,
			message,
		}
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

impl From<http::Response<Bytes>> for Response {
	fn from(res: http::Response<Bytes>) -> Self {
		let (parts, body) = res.into_parts();
		Response {
			parts,
			body: body.into(),
		}
	}
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
			None => Response::not_found(),
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
