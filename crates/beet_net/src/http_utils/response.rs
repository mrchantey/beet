//! Generic response type for routing.
//!
//! The [`Response`] type abstracts over different transport mechanisms,
//! allowing the same routing infrastructure to return responses for
//! HTTP requests, CLI commands, and REPL output.
//!
//! # Example
//!
//! ```
//! # use beet_net::prelude::*;
//! # use http::StatusCode;
//! // Create an HTTP-style response
//! let response = Response::ok().with_body("Hello, world!");
//!
//! // Create error responses
//! let not_found = Response::not_found();
//! let error = Response::from_status(StatusCode::INTERNAL_SERVER_ERROR);
//! ```

use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use http::StatusCode;
use std::convert::Infallible;

/// A generalized response type that can represent HTTP responses, CLI output,
/// or other request-response patterns.
///
/// This is a [`Component`] that is added to route entities after processing.
/// It contains both the response metadata ([`ResponseParts`]) and the body.
///
/// # Deref
///
/// `Response` implements `Deref<Target = ResponseParts>`, so all methods on
/// [`ResponseParts`] and [`Parts`] are available directly:
///
/// ```
/// # use beet_net::prelude::*;
/// # use http::StatusCode;
/// let response = Response::ok();
/// assert_eq!(response.status(), StatusCode::OK);  // From ResponseParts
/// ```
#[derive(Debug, Component)]
pub struct Response {
	parts: ResponseParts,
	pub body: Body,
}

impl std::error::Error for Response {}

impl std::fmt::Display for Response {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			formatter,
			"Response - Status: {}, Message: '{}'",
			self.parts.status(),
			match &self.body {
				Body::Stream(_) => "<stream>".into(),
				Body::Bytes(bytes) =>
					String::from_utf8_lossy(bytes).to_string(),
			}
		)
	}
}
/// Equality check for Response based on body and status code
impl PartialEq for Response {
	fn eq(&self, other: &Self) -> bool {
		self.body.bytes_eq(&other.body)
			&& self.parts.status() == other.parts.status()
	}
}

impl std::ops::Deref for Response {
	type Target = ResponseParts;
	fn deref(&self) -> &Self::Target { &self.parts }
}

impl std::ops::DerefMut for Response {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.parts }
}

impl Response {
	/// Creates an OK (200) response
	pub fn ok() -> Self { Self::from_status(StatusCode::OK) }

	/// Creates a Not Found (404) response
	pub fn not_found() -> Self { Self::from_status(StatusCode::NOT_FOUND) }

	/// Creates a Temporary Redirect (307) response with the given location
	pub fn temporary_redirect(location: impl Into<String>) -> Self {
		let mut parts = ResponseParts::new(StatusCode::TEMPORARY_REDIRECT);
		parts.parts_mut().insert_header("location", location.into());
		Self {
			parts,
			body: Default::default(),
		}
	}

	/// Creates a Permanent Redirect (301) response with the given location
	pub fn permanent_redirect(location: impl Into<String>) -> Self {
		let mut parts = ResponseParts::new(StatusCode::MOVED_PERMANENTLY);
		parts.parts_mut().insert_header("location", location.into());
		Self {
			parts,
			body: Default::default(),
		}
	}

	/// Returns the status code
	pub fn status(&self) -> StatusCode { self.parts.status() }

	/// Creates a response with the given status code
	pub fn from_status(status: StatusCode) -> Self {
		Self {
			parts: ResponseParts::new(status),
			body: Default::default(),
		}
	}

	/// Sets the response body
	pub fn with_body(mut self, body: impl Into<Body>) -> Self {
		self.body = body.into();
		self
	}

	/// Creates a response with status, body, and content type
	pub fn from_status_body(
		status: StatusCode,
		body: impl AsRef<[u8]>,
		content_type: &str,
	) -> Self {
		let mut parts = ResponseParts::new(status);
		parts
			.parts_mut()
			.insert_header("content-type", content_type);
		Self {
			parts,
			body: Bytes::copy_from_slice(body.as_ref()).into(),
		}
	}

	/// Gets a header value by name
	pub fn header(
		&self,
		header: http::header::HeaderName,
	) -> Result<Option<&str>> {
		match self.parts.get_header(header.as_str()) {
			Some(value) => Ok(Some(value.as_str())),
			None => Ok(None),
		}
	}

	/// Check whether a header exactly matches the given value.
	/// Do not use this for checks like `Content-Type` as they may
	/// have additional parameters like `application/json; charset=utf-8`.
	pub fn header_matches(
		&self,
		header: http::header::HeaderName,
		value: &str,
	) -> bool {
		self.parts
			.get_header(header.as_str())
			.map_or(false, |val| val == value)
	}

	/// Check whether a header contains the given value. Use this for
	/// checks like `Content-Type` where the value may have additional parameters
	/// like `application/json; charset=utf-8`.
	pub fn header_contains(
		&self,
		header: http::header::HeaderName,
		value: &str,
	) -> bool {
		self.parts
			.get_header(header.as_str())
			.map_or(false, |val| val.contains(value))
	}

	/// Creates a response from parts and body
	pub fn from_parts(parts: ResponseParts, body: Bytes) -> Self {
		Self {
			parts,
			body: body.into(),
		}
	}

	/// Creates a response from http parts and body
	pub fn from_http_parts(parts: http::response::Parts, body: Bytes) -> Self {
		Self {
			parts: ResponseParts::from(parts),
			body: body.into(),
		}
	}

	/// Create a response with the given body and content type
	pub fn ok_body(body: impl Into<Body>, content_type: &str) -> Self {
		let mut parts = ResponseParts::ok();
		parts
			.parts_mut()
			.insert_header("content-type", content_type);
		Self {
			parts,
			body: body.into(),
		}
	}

	/// Create a response with the given body, guessing the content type
	/// based on the file extension, defaulting to `application/octet-stream`
	/// if the extension is not recognized.
	pub fn ok_mime_guess(
		body: impl Into<Body>,
		path: impl AsRef<std::path::Path>,
	) -> Self {
		let mime_type = mime_guess::from_path(path).first_or_octet_stream();
		Self::ok_body(body, mime_type.as_ref())
	}

	/// Returns a reference to the response parts
	pub fn parts(&self) -> &ResponseParts { &self.parts }

	/// Returns a mutable reference to the response parts
	pub fn parts_mut(&mut self) -> &mut ResponseParts { &mut self.parts }

	/// Consumes the response and returns the parts and body
	pub fn into_parts(self) -> (ResponseParts, Body) { (self.parts, self.body) }

	/// Consumes the response body and returns it as bytes
	pub async fn bytes(self) -> Result<Bytes> { self.body.into_bytes().await }

	/// Consumes the response body and returns it as a string
	pub async fn text(self) -> Result<String> { self.body.into_string().await }

	/// Consumes the response body and parses it as JSON
	#[cfg(feature = "serde")]
	pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
		self.body.into_json().await
	}

	/// Converts this response into an http::Response
	pub async fn into_http(self) -> Result<http::Response<Bytes>> {
		let bytes = self.body.into_bytes().await?;
		let http_parts: http::response::Parts = self.parts.try_into()?;
		http::Response::from_parts(http_parts, bytes).xok()
	}

	/// Convert a response that completed but may have returned a non-2xx status code into a result,
	/// returning an error if the status code is not successful 2xx.
	pub async fn into_result(self) -> Result<Self, HttpError> {
		if self.parts.status().is_success() {
			Ok(self)
		} else {
			Err(self.into_error().await)
		}
	}

	/// Convert the response into an error even if it is a 2xx status code,
	/// extracting the status code and message from the body.
	/// For a method that checks the status code see [`Response::into_result`].
	pub async fn into_error(self) -> HttpError {
		let status = self.status();
		let Ok(bytes) = self.body.into_bytes().await else {
			return HttpError::internal_error("Failed to read response body");
		};
		let message = if !bytes.is_empty() {
			String::from_utf8_lossy(&bytes).to_string()
		} else {
			Default::default()
		};

		HttpError {
			status_code: status,
			message,
		}
	}

	/// Adds a header to the response
	pub fn with_header(mut self, key: &str, value: &str) -> Self {
		self.parts.parts_mut().insert_header(key, value);
		self
	}

	/// Sets the content type header
	pub fn with_content_type(self, content_type: &str) -> Self {
		self.with_header("content-type", content_type)
	}
}

impl From<http::Response<Body>> for Response {
	fn from(res: http::Response<Body>) -> Self {
		let (parts, body) = res.into_parts();
		Response {
			parts: ResponseParts::from(parts),
			body,
		}
	}
}

impl From<http::Response<Bytes>> for Response {
	fn from(res: http::Response<Bytes>) -> Self {
		let (parts, body) = res.into_parts();
		Response {
			parts: ResponseParts::from(parts),
			body: body.into(),
		}
	}
}

/// Allows for blanket implementation of `Into<Response>`,
/// including `Result<T,E>` where `T` and `E` both implement `IntoResponse`
/// and Option<T> where `T` implements `IntoResponse`, and [`None`] is not found.
pub trait IntoResponse<M> {
	fn into_response(self) -> Response;
}

impl<T: IntoResponse<M1>, M1, E: IntoResponse<M2>, M2>
	IntoResponse<(Self, M1, M2)> for Result<T, E>
{
	fn into_response(self) -> Response {
		match self {
			Ok(val) => val.into_response(),
			Err(err) => err.into_response(),
		}
	}
}

impl Into<Response> for BevyError {
	fn into(self) -> Response { HttpError::from_opaque(self).into() }
}

impl IntoResponse<Self> for Bytes {
	fn into_response(self) -> Response {
		Response::ok_body(self, "application/octet-stream")
	}
}

impl IntoResponse<Self> for &[u8] {
	fn into_response(self) -> Response {
		Response::ok_body(
			Bytes::copy_from_slice(self),
			"application/octet-stream",
		)
	}
}

impl IntoResponse<Self> for Infallible {
	fn into_response(self) -> Response {
		unreachable!("Infallible cannot be converted to a response");
	}
}

impl IntoResponse<Self> for () {
	fn into_response(self) -> Response { Response::ok() }
}

impl IntoResponse<Self> for StatusCode {
	fn into_response(self) -> Response { Response::from_status(self) }
}

impl<T: TryInto<Response>, M1> IntoResponse<(Self, M1)> for T
where
	T::Error: IntoResponse<M1>,
{
	fn into_response(self) -> Response {
		match self.try_into() {
			Ok(response) => response,
			Err(err) => err.into_response(),
		}
	}
}

/// None = not found, matching http principles ie crud operations
impl<T: IntoResponse<M>, M> IntoResponse<(Self, M)> for Option<T> {
	fn into_response(self) -> Response {
		match self {
			Some(val) => val.into_response(),
			None => Response::not_found(),
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn response_ok() {
		let response = Response::ok();
		response.status().xpect_eq(StatusCode::OK);
	}

	#[test]
	fn response_not_found() {
		let response = Response::not_found();
		response.status().xpect_eq(StatusCode::NOT_FOUND);
	}

	#[test]
	fn response_from_status() {
		let response = Response::from_status(StatusCode::CREATED);
		response.status().xpect_eq(StatusCode::CREATED);
	}

	#[test]
	fn response_with_body() {
		let response = Response::ok().with_body("hello");
		response
			.body
			.bytes_eq(&Body::Bytes(Bytes::from("hello")))
			.xpect_true();
	}

	#[test]
	fn response_from_status_body() {
		let response =
			Response::from_status_body(StatusCode::OK, b"data", "text/plain");
		response.status().xpect_eq(StatusCode::OK);
		response
			.header_contains(http::header::CONTENT_TYPE, "text/plain")
			.xpect_true();
	}

	#[test]
	fn response_deref_to_parts() {
		let response = Response::ok();
		// Should be able to call ResponseParts methods via Deref
		response.status().xpect_eq(StatusCode::OK);
	}

	#[test]
	fn response_ok_body() {
		let response = Response::ok_body("hello", "text/plain");
		response.status().xpect_eq(StatusCode::OK);
		response
			.header_contains(http::header::CONTENT_TYPE, "text/plain")
			.xpect_true();
	}

	#[test]
	fn response_temporary_redirect() {
		let response = Response::temporary_redirect("/new-location");
		response.status().xpect_eq(StatusCode::TEMPORARY_REDIRECT);
		response
			.get_header("location")
			.unwrap()
			.xpect_eq("/new-location");
	}

	#[test]
	fn response_permanent_redirect() {
		let response = Response::permanent_redirect("/new-location");
		response.status().xpect_eq(StatusCode::MOVED_PERMANENTLY);
		response
			.get_header("location")
			.unwrap()
			.xpect_eq("/new-location");
	}

	#[test]
	fn response_with_header() {
		let response = Response::ok().with_header("x-custom", "value");
		response.get_header("x-custom").unwrap().xpect_eq("value");
	}

	#[test]
	fn response_into_parts() {
		let response = Response::ok().with_body("data");
		let (parts, body) = response.into_parts();

		parts.status().xpect_eq(StatusCode::OK);
		body.bytes_eq(&Body::Bytes(Bytes::from("data")))
			.xpect_true();
	}

	#[test]
	fn response_partial_eq() {
		let response1 = Response::ok().with_body("hello");
		let response2 = Response::ok().with_body("hello");
		let response3 = Response::ok().with_body("world");

		(response1 == response2).xpect_true();
		(response2 == response3).xpect_false();
	}

	#[test]
	fn response_display() {
		let response = Response::ok().with_body("hello");
		let display = format!("{}", response);
		display.clone().xpect_contains("200");
		display.xpect_contains("hello");
	}

	#[test]
	fn into_response_unit() {
		let response = ().into_response();
		response.status().xpect_eq(StatusCode::OK);
	}

	#[test]
	fn into_response_status_code() {
		let response = StatusCode::CREATED.into_response();
		response.status().xpect_eq(StatusCode::CREATED);
	}

	#[test]
	fn into_response_option_some() {
		let response = Some(StatusCode::CREATED).into_response();
		response.status().xpect_eq(StatusCode::CREATED);
	}

	#[test]
	fn into_response_option_none() {
		let response: Response = None::<StatusCode>.into_response();
		response.status().xpect_eq(StatusCode::NOT_FOUND);
	}
}
