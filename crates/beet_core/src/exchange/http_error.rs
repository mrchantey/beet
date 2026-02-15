//! HTTP error type for request/response handling.
//!
//! This module provides [`HttpError`], an error type designed for HTTP-style
//! request handling that includes a status code and message.
//!
//! # Security
//!
//! In release builds, internal error details are logged but an opaque error
//! is returned to clients to prevent information leakage.

use crate::prelude::*;
use bevy::ecs::system::RegisteredSystemError;
use bevy::ecs::system::RunSystemError;
use tracing::error;

/// Result type alias using [`HttpError`].
pub type HttpResult<T> = std::result::Result<T, HttpError>;


/// A non-200 response from an HTTP request.
///
/// The message *will be* returned to the client, so ensure that no sensitive
/// information is included. By default, non-HTTP [`BevyError`] messages will be
/// logged and an opaque error will be returned to the client in release builds.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HttpError {
	/// The HTTP status code.
	#[cfg_attr(feature = "serde", serde(with = "status_code_serde"))]
	pub status_code: StatusCode,
	/// The error message.
	pub message: String,
}


impl HttpError {
	/// Creates an [`HttpError`] with the given status code and an empty message.
	pub fn from_status(status_code: StatusCode) -> Self {
		Self {
			status_code,
			message: Default::default(),
		}
	}

	/// Creates a new [`HttpError`] with the given status code and message.
	pub fn new(status_code: StatusCode, message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
			status_code,
		}
	}

	/// Creates a 404 Not Found error.
	pub fn not_found() -> Self { Self::from_status(StatusCode::NotFound) }

	/// Creates a 400 Bad Request error with the given message.
	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::new(StatusCode::MalformedRequest, message)
	}

	/// Creates a 500 Internal Server Error.
	///
	/// In debug builds, the full error message is included.
	/// In release builds, the error is logged but an opaque message is returned.
	pub fn internal_error(message: impl Into<String>) -> Self {
		let message = message.into();
		// we are about to lose the internal bevy message, so log it
		// and return an opaque error
		error!("Internal Error: {}", message);
		#[cfg(debug_assertions)]
		{
			Self::new(
				StatusCode::InternalError,
				format!(
					"Internal Error: {}\n\nThis error will *not* be returned to the client in release builds.",
					message
				),
			)
		}
		#[cfg(not(debug_assertions))]
		{
			Self::new(StatusCode::InternalError, format!("Internal Error"))
		}
	}

	/// Converts a [`BevyError`] into an [`HttpError`].
	///
	/// If the error is already an [`HttpError`], it is returned directly.
	/// Otherwise, the error is logged and an opaque internal server error
	/// is returned in release builds.
	pub fn from_opaque(error: impl Into<BevyError>) -> Self {
		let error = error.into();
		if let Some(inner) = error.downcast_ref::<HttpError>() {
			// If the error is already an HttpError, return it directly
			inner.clone()
		} else {
			let error = error.to_string();
			Self::internal_error(error)
		}
	}
}
impl std::error::Error for HttpError {}

impl std::fmt::Display for HttpError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if self.message.is_empty() {
			write!(f, "{}", self.status_code)
		} else {
			write!(f, "{}: {}", self.status_code, self.message)
		}
	}
}
impl From<BevyError> for HttpError {
	fn from(err: BevyError) -> HttpError { Self::from_opaque(err) }
}
impl From<RunSystemError> for HttpError {
	fn from(err: RunSystemError) -> HttpError {
		Self::internal_error(err.to_string())
	}
}
impl<In, Out> From<RegisteredSystemError<In, Out>> for HttpError
where
	In: 'static + SystemInput,
	Out: 'static,
{
	fn from(err: RegisteredSystemError<In, Out>) -> HttpError {
		Self::from_opaque(err)
	}
}

#[cfg(feature = "json")]
impl From<serde_json::Error> for HttpError {
	fn from(err: serde_json::Error) -> HttpError { Self::from_opaque(err) }
}

#[cfg(feature = "serde")]
impl From<serde_urlencoded::de::Error> for HttpError {
	fn from(err: serde_urlencoded::de::Error) -> HttpError {
		Self::from_opaque(err)
	}
}

impl From<HttpError> for Response {
	fn from(error: HttpError) -> Response {
		if error.message.is_empty() {
			Response::from_status(error.status_code)
		} else {
			Response::from_status_body(
				error.status_code,
				error.message.as_bytes(),
				"text/plain; charset=utf-8",
			)
		}
	}
}


/// Serde support for [`StatusCode`].
#[cfg(feature = "serde")]
pub mod status_code_serde {
	use super::*;
	use serde::Deserialize;

	/// Serializes a [`StatusCode`] to a numeric value.
	pub fn serialize<S>(
		status: &StatusCode,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		#[cfg(feature = "http")]
		{
			let http_status: http::StatusCode = (*status).into();
			serializer.serialize_u16(http_status.as_u16())
		}
		#[cfg(not(feature = "http"))]
		{
			let exit_code: u8 = (*status).into();
			serializer.serialize_u8(exit_code)
		}
	}

	/// Deserializes a [`StatusCode`] from a numeric value.
	#[cfg(feature = "serde")]
	pub fn deserialize<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		#[cfg(feature = "http")]
		{
			let code = u16::deserialize(deserializer)?;
			Ok(StatusCode::from_http_raw(code))
		}
		#[cfg(not(feature = "http"))]
		{
			let code = u8::deserialize(deserializer)?;
			Ok(StatusCode::from(code))
		}
	}
}
