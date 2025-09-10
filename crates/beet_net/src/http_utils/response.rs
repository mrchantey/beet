use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::utils::PipelineTarget;
use bevy::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use bytes::Bytes;
use futures::Stream;
use http::StatusCode;
use http::response;
use send_wrapper::SendWrapper;
use std::convert::Infallible;
use std::pin::Pin;


#[cfg(target_arch = "wasm32")]
type DynBytesStream = dyn Stream<Item = Result<Bytes>>;
/// Axum requires Stream to be Send
#[cfg(not(target_arch = "wasm32"))]
type DynBytesStream = dyn Stream<Item = Result<Bytes>> + Send;

/// Added by the route or its layers, otherwise an empty [`StatusCode::Ok`]
/// will be returned.
#[derive(Debug, Resource)]
pub struct Response {
	pub parts: response::Parts,
	pub body: Body,
}

pub enum Body {
	Bytes(Bytes),
	Stream(SendWrapper<Pin<Box<DynBytesStream>>>),
}

impl Stream for Body {
	type Item = Result<Bytes>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		match &mut *self {
			Body::Bytes(bytes) => {
				if !bytes.is_empty() {
					let taken = std::mem::take(bytes);
					std::task::Poll::Ready(Some(Ok(taken)))
				} else {
					std::task::Poll::Ready(None)
				}
			}
			Body::Stream(stream) => Pin::new(stream).poll_next(cx),
		}
	}
}

impl Into<Body> for Bytes {
	fn into(self) -> Body { Body::Bytes(self) }
}

impl std::fmt::Debug for Body {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Body::Bytes(bytes) => write!(f, "Body::Bytes({:?})", bytes),
			Body::Stream(_) => write!(f, "Body::Stream(...)"),
		}
	}
}

impl Body {
	/// Any body with a content length greater than this will be parsed as a stream.
	pub const MAX_BUFFER_SIZE: usize = 1 * 1024 * 1024; // 1 MB

	pub async fn into_bytes(mut self) -> Result<Bytes> {
		match self {
			Body::Bytes(bytes) => Ok(bytes),
			Body::Stream(_) => {
				let mut buffer = bytes::BytesMut::new();
				while let Some(chunk) = self.next().await? {
					buffer.extend_from_slice(&chunk);
				}
				Ok(buffer.freeze())
			}
		}
	}

	pub async fn next(&mut self) -> Result<Option<Bytes>> {
		match self {
			Body::Bytes(bytes) if !bytes.is_empty() => {
				Ok(Some(std::mem::take(bytes)))
			}
			Body::Bytes(_) => Ok(None),
			Body::Stream(stream) => match stream.next().await {
				Some(result) => Ok(Some(result?)),
				None => Ok(None),
			},
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
	pub fn temporary_redirect(location: impl Into<String>) -> Self {
		Self::from_parts(
			http::response::Builder::new()
				.status(StatusCode::TEMPORARY_REDIRECT)
				.header(http::header::LOCATION, location.into())
				.body(())
				.unwrap()
				.into_parts()
				.0,
			Default::default(),
		)
	}
	/// Create a response with a 301 MOVED_PERMANENTLY status code
	pub fn permanent_redirect(location: impl Into<String>) -> Self {
		Self::from_parts(
			http::response::Builder::new()
				.status(StatusCode::MOVED_PERMANENTLY)
				.header(http::header::LOCATION, location.into())
				.body(())
				.unwrap()
				.into_parts()
				.0,
			Default::default(),
			// "Redirecting...".into(),// does that produce fouc?
		)
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

	pub fn header(
		&self,
		header: http::header::HeaderName,
	) -> Result<Option<&str>> {
		match self.parts.headers.get(&header) {
			Some(value) => Ok(Some(value.to_str()?)),
			None => Ok(None),
		}
	}

	/// Check whether a header exactly matches the given value,
	/// do not use this for checks like `Content-Type` as they may
	/// have additional parameters like `application/json; charset=utf-8`.
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
	/// Check whether a header contains the given value, use this for
	/// checks like `Content-Type` where the value may have additional parameters
	/// like `application/json; charset=utf-8`.
	pub fn header_contains(
		&self,
		header: http::header::HeaderName,
		value: &str,
	) -> bool {
		self.parts
			.headers
			.get(&header)
			.map_or(false, |v| v.to_str().map_or(false, |s| s.contains(value)))
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

	/// Create a response with the given body, guessing the content type
	/// based on the file extension, defaulting to `application/octet-stream`
	/// if the extension is not recognized.
	pub fn ok_mime_guess(
		body: impl AsRef<[u8]>,
		path: impl AsRef<std::path::Path>,
	) -> Self {
		let mime_type = mime_guess::from_path(path).first_or_octet_stream();
		Self::ok_body(body, mime_type.as_ref())
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
		// let is_text = self.header_contains(CONTENT_TYPE, "text/plain");
		let status = self.status();
		let Ok(bytes) = self.body.into_bytes().await else {
			return HttpError::internal_error("Failed to read response body");
		};
		let message = if !bytes.is_empty() {
			// let message = if is_text && !bytes.is_empty() {
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


impl Into<Response> for BevyError {
	fn into(self) -> Response { HttpError::from_opaque(self).into() }
}

// impl Into<Response> for RunSystemError {
// 	fn into(self) -> Response { HttpError::from_opaque(self).into() }
// }

// impl<T> IntoResponse for T where T:Into<HttpError> {
// 	fn into_response(self) -> Response {
// 		let error: HttpError = self.into();
// 		error.into()
// 	}
// }

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

impl From<http::Response<Body>> for Response {
	fn from(res: http::Response<Body>) -> Self {
		let (parts, body) = res.into_parts();
		Response { parts, body }
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

impl IntoResponse for () {
	fn into_response(self) -> Response { Response::ok() }
}

impl IntoResponse for StatusCode {
	fn into_response(self) -> Response { Response::from_status(self) }
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
