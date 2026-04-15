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
//! // Create an HTTP-style response
//! let response = Response::ok().with_body("Hello, world!");
//!
//! // Create error responses
//! let not_found = Response::not_found();
//! let error = Response::from_status(StatusCode::INTERNAL_SERVER_ERROR);
//! ```

use super::*;
use beet_core::prelude::*;
use bytes::Bytes;
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
/// [`ResponseParts`] are available directly:
///
/// ```
/// # use beet_net::prelude::*;
/// let response = Response::ok();
/// assert_eq!(response.status(), StatusCode::OK);  // From ResponseParts
/// ```
#[derive(Debug, Component)]
#[require(ResponseMarker = ResponseMarker{_sealed:()})]
pub struct Response {
	/// The response metadata including status code and headers.
	pub parts: ResponseParts,
	/// The response body, which may be bytes or a stream.
	pub body: Body,
}


/// Marker component to indicate that a response has been inserted.
/// Even if the response gets taken this struct should remain.
#[derive(Debug, Component)]
pub struct ResponseMarker {
	_sealed: (),
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
	/// Creates a new response with the given parts and body.
	pub fn new(parts: ResponseParts, body: Body) -> Self {
		Self { parts, body }
	}

	/// Creates an OK (200) response
	pub fn ok() -> Self { Self::from_status(StatusCode::OK) }

	/// Creates a Not Found (404) response
	pub fn not_found() -> Self { Self::from_status(StatusCode::NOT_FOUND) }
	/// Creates an Internal Server Error (500) response
	pub fn internal_error() -> Self {
		Self::from_status(StatusCode::INTERNAL_SERVER_ERROR)
	}

	/// Creates a Temporary Redirect (307) response with the given location
	pub fn temporary_redirect(location: impl Into<String>) -> Self {
		let mut parts = ResponseParts::new(StatusCode::TEMPORARY_REDIRECT);
		parts.headers.set::<header::Location>(location.into());
		Self {
			parts,
			body: Default::default(),
		}
	}

	/// Creates a Permanent Redirect (301) response with the given location
	pub fn permanent_redirect(location: impl Into<String>) -> Self {
		let mut parts = ResponseParts::new(StatusCode::MOVED_PERMANENTLY);
		parts.headers.set::<header::Location>(location.into());
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

	/// Creates an OK response with a JSON-serialized body and `content-type` header.
	///
	/// ```
	/// # use beet_net::prelude::*;
	/// # use beet_net::headers;
	/// let response = Response::with_json(&serde_json::json!({"foo": 42})).unwrap();
	/// assert_eq!(response.status(), StatusCode::OK);
	/// let media_type = response.parts.headers.get::<headers::ContentType>().unwrap().unwrap();
	/// assert_eq!(media_type, MediaType::Json);
	/// ```
	#[cfg(feature = "json")]
	pub fn with_json<T: serde::Serialize>(value: &T) -> Result<Self> {
		let body = Body::from_json(value)?;
		Self::ok()
			.with_body(body)
			.with_content_type(MediaType::Json)
			.xok()
	}

	/// Creates an OK response with a raw JSON string body and `content-type` header.
	///
	/// ```
	/// # use beet_net::prelude::*;
	/// # use beet_net::headers;
	/// let response = Response::with_json_str(r#"{"foo": 42}"#);
	/// let media_type = response.parts.headers.get::<headers::ContentType>().unwrap().unwrap();
	/// assert_eq!(media_type, MediaType::Json);
	/// ```
	#[cfg(feature = "json")]
	pub fn with_json_str(json: impl AsRef<str>) -> Self {
		Self::ok()
			.with_body(json.as_ref())
			.with_content_type(MediaType::Json)
	}

	/// Deserializes the response body using the format indicated by
	/// the `content-type` header, defaulting to JSON.
	///
	/// ```ignore
	/// # use beet_net::prelude::*;
	/// # async {
	/// let response = Response::with_json(&42u32).unwrap();
	/// let value: u32 = response.deserialize().await.unwrap();
	/// assert_eq!(value, 42);
	/// # };
	/// ```
	#[cfg(feature = "serde")]
	pub async fn deserialize<T: serde::de::DeserializeOwned>(
		self,
	) -> Result<T> {
		let media_type = self
			.parts
			.headers
			.get::<header::ContentType>()
			.and_then(|res| res.ok())
			.unwrap_or(MediaType::Json);
		self.body.into_media_type(media_type).await
	}

	/// Deserializes the response body using the format indicated by
	/// the `content-type` header, blocking the current thread.
	///
	/// ```ignore
	/// # use beet_net::prelude::*;
	/// let response = Response::with_json(&42u32).unwrap();
	/// let value: u32 = response.deserialize_blocking().unwrap();
	/// assert_eq!(value, 42);
	/// ```
	#[cfg(feature = "serde")]
	pub fn deserialize_blocking<T: serde::de::DeserializeOwned>(
		self,
	) -> Result<T> {
		async_ext::block_on(self.deserialize())
	}

	/// Creates a response with status, body, and content type
	pub fn from_status_body(
		status: StatusCode,
		body: impl AsRef<[u8]>,
		content_type: MediaType,
	) -> Self {
		let mut parts = ResponseParts::new(status);
		parts.headers.set_content_type(content_type);
		Self {
			parts,
			body: Bytes::copy_from_slice(body.as_ref()).into(),
		}
	}

	/// Creates a response from parts and body
	pub fn from_parts(parts: ResponseParts, body: Bytes) -> Self {
		Self {
			parts,
			body: body.into(),
		}
	}

	/// Creates a response from http parts and body
	#[cfg(feature = "http")]
	pub fn from_http_parts(parts: http::response::Parts, body: Bytes) -> Self {
		Self {
			parts: ResponseParts::from(parts),
			body: body.into(),
		}
	}

	/// Create a response with the given body and content type
	pub fn ok_body(body: impl Into<Body>, content_type: MediaType) -> Self {
		let mut parts = ResponseParts::ok();
		parts.headers.set_content_type(content_type);
		Self {
			parts,
			body: body.into(),
		}
	}

	/// Create a response with the given body, inferring the content type
	/// from the file extension via [`MediaType::from_path`].
	/// Defaults to `application/octet-stream` for unrecognized extensions.
	pub fn ok_from_path(
		body: impl Into<Body>,
		path: impl AsRef<std::path::Path>,
	) -> Self {
		let media_type = MediaType::from_path(path);
		Self::ok_body(body, media_type)
	}

	/// Returns a reference to the response parts
	pub fn response_parts(&self) -> &ResponseParts { &self.parts }

	/// Returns a mutable reference to the response parts
	pub fn response_parts_mut(&mut self) -> &mut ResponseParts {
		&mut self.parts
	}

	/// Consumes the response and returns the parts and body
	pub fn into_parts(self) -> (ResponseParts, Body) { (self.parts, self.body) }

	/// Consumes the response body and returns it as bytes
	pub async fn bytes(self) -> Result<Bytes> { self.body.into_bytes().await }

	/// Consumes the response body and returns it as bytes
	pub async fn bytes_vec(self) -> Result<Vec<u8>> {
		self.bytes().await.map(|b| b.to_vec())
	}
	/// Consumes the response body and returns it as [`MediaBytes`],
	/// using the [`header::ContentType`], or defaulting to [`MediaType::Bytes`].
	/// Note, the bytes may be empty.
	pub async fn into_media_bytes(self) -> Result<MediaBytes> {
		let media_type = self
			.parts
			.headers
			.get::<header::ContentType>()
			.and_then(|res| res.ok())
			.unwrap_or(MediaType::Bytes);
		let bytes = self.bytes_vec().await?;
		Ok(MediaBytes::new(media_type, bytes))
	}

	/// Consumes the response body and returns it as a string
	pub async fn text(self) -> Result<String> { self.body.into_string().await }

	/// Consumes the response body and parses it as JSON
	#[cfg(feature = "json")]
	pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T> {
		self.body.into_json().await
	}

	/// Converts this response into an http::Response
	#[cfg(feature = "http")]
	pub async fn into_http(self) -> Result<http::Response<Bytes>> {
		let bytes = self.body.into_bytes().await?;
		let http_parts: http::response::Parts = self.parts.try_into()?;
		http::Response::from_parts(http_parts, bytes).xok()
	}

	/// Convert a response that completed but may have returned a non-2xx status code into a result,
	/// returning an error if the status code is not successful 2xx.
	pub async fn into_result(self) -> Result<Self, HttpError> {
		if self.parts.status().is_ok() {
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
		self.parts.headers.set_raw(key, value);
		// NOTE: `with_header` accepts arbitrary key/value strings — raw is correct here.
		self
	}

	/// Sets the content type header.
	pub fn with_content_type(mut self, content_type: MediaType) -> Self {
		self.parts.headers.set_content_type(content_type);
		self
	}

	/// Sets the body and content type based on the given media bytes.
	pub fn with_media(self, bytes: MediaBytes) -> Self {
		let (media_type, bytes) = bytes.take();
		self.with_content_type(media_type).with_body(bytes)
	}

	/// Unwrap the ok status code and get the body as text
	pub async fn unwrap_str(self) -> String {
		self.into_result().await.unwrap().text().await.unwrap()
	}
}

#[cfg(feature = "http")]
impl From<http::Response<Body>> for Response {
	fn from(res: http::Response<Body>) -> Self {
		let (parts, body) = res.into_parts();
		Response {
			parts: ResponseParts::from(parts),
			body,
		}
	}
}

#[cfg(feature = "http")]
impl From<http::Response<Bytes>> for Response {
	fn from(res: http::Response<Bytes>) -> Self {
		let (parts, body) = res.into_parts();
		Response {
			parts: ResponseParts::from(parts),
			body: body.into(),
		}
	}
}

#[cfg(feature = "http")]
impl From<http::StatusCode> for Response {
	fn from(status: http::StatusCode) -> Self {
		Response::from_status(StatusCode::from(status))
	}
}

impl IntoResponse<Self> for StatusCode {
	fn into_response(self) -> Response { Response::from_status(self) }
}


/// Converts a type into a [`Response`].
///
/// This trait enables blanket implementations for common types:
/// - `Result<T, E>` where both `T` and `E` implement `IntoResponse`
/// - `Option<T>` where `T` implements `IntoResponse` ([`None`] becomes 404)
pub trait IntoResponse<M> {
	/// Converts this type into a response.
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

impl From<BevyError> for Response {
	fn from(value: BevyError) -> Response {
		HttpError::from_opaque(value).into()
	}
}

impl IntoResponse<Self> for Bytes {
	fn into_response(self) -> Response {
		Response::ok_body(self, MediaType::Bytes)
	}
}

impl IntoResponse<Self> for &[u8] {
	fn into_response(self) -> Response {
		Response::ok_body(Bytes::copy_from_slice(self), MediaType::Bytes)
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
// impl<T: Into<Response>> IntoResponse<Self> for T {
// 	fn into_response(self) -> Response { self.into() }
// }


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


#[cfg(test)]
mod test {
	#[allow(unused_imports)]
	use super::*;


	#[test]
	fn response_ok() { Response::ok().status().xpect_eq(StatusCode::OK); }

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
		let response = Response::from_status_body(
			StatusCode::OK,
			b"data",
			MediaType::Text,
		);
		response.status().xpect_eq(StatusCode::OK);
		response
			.headers
			.get::<headers::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Text);
	}

	#[test]
	fn response_deref_to_parts() {
		let response = Response::ok();
		response.status().xpect_eq(StatusCode::OK);
	}


	#[test]
	fn response_temporary_redirect() {
		let response = Response::temporary_redirect("/new-location");
		response.status().xpect_eq(StatusCode::TEMPORARY_REDIRECT);
		response
			.parts
			.headers
			.get::<header::Location>()
			.unwrap()
			.unwrap()
			.xpect_eq("/new-location");
	}

	#[test]
	fn response_permanent_redirect() {
		let response = Response::permanent_redirect("/new-location");
		response.status().xpect_eq(StatusCode::MOVED_PERMANENTLY);
		response
			.parts
			.headers
			.get::<header::Location>()
			.unwrap()
			.unwrap()
			.xpect_eq("/new-location");
	}

	#[test]
	fn response_with_header() {
		let response = Response::ok().with_header("x-custom", "value");
		response
			.parts
			.headers
			.first_raw("x-custom")
			.unwrap()
			.xpect_eq("value");
		// NOTE: x-custom has no typed header — raw access is correct here.
	}

	#[cfg(feature = "json")]
	#[test]
	fn response_with_json() {
		use serde::Deserialize;
		use serde::Serialize;

		#[derive(Debug, PartialEq, Serialize, Deserialize)]
		struct Payload {
			foo: u32,
		}

		let payload = Payload { foo: 42 };
		let response = Response::with_json(&payload).unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response
			.parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);

		let body_bytes = response.body.try_into_bytes().unwrap();
		let roundtrip: Payload = serde_json::from_slice(&body_bytes).unwrap();
		roundtrip.xpect_eq(payload);
	}

	#[cfg(feature = "json")]
	#[test]
	fn response_with_json_str() {
		let response = Response::with_json_str(r#"{"foo":42}"#);
		response.status().xpect_eq(StatusCode::OK);
		response
			.parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);

		let body_bytes = response.body.try_into_bytes().unwrap();
		String::from_utf8(body_bytes.to_vec())
			.unwrap()
			.xpect_eq(r#"{"foo":42}"#);
	}

	#[cfg(feature = "json")]
	#[test]
	fn response_with_media() {
		let mb = MediaBytes::serialize(MediaType::Json, &42u32).unwrap();
		let response = Response::ok().with_media(mb);
		response.status().xpect_eq(StatusCode::OK);
		response
			.parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
	}

	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn response_deserialize_json() {
		use serde::Deserialize;
		use serde::Serialize;

		#[derive(Debug, PartialEq, Serialize, Deserialize)]
		struct Payload {
			foo: u32,
		}

		let payload = Payload { foo: 30 };
		let response = Response::with_json(&payload).unwrap();
		let roundtrip: Payload = response.deserialize().await.unwrap();
		roundtrip.xpect_eq(payload);
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
		display.clone().xpect_contains("OK");
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
}
