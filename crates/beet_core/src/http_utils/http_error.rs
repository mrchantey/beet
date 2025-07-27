use crate::prelude::*;
use bevy::ecs::system::RegisteredSystemError;
use bevy::ecs::system::RunSystemError;
use bevy::prelude::*;
use http::StatusCode;
use tracing::error;

pub type HttpResult<T> = std::result::Result<T, HttpError>;


/// A non-200 response from a http request.
/// The message *will be* returned to the client so ensure that no sensitive information is included.
/// By default non http [`BevyError`] messages will be logged and an opaque error will be returned to the client
/// in release builds
#[derive(Debug, Clone)]
pub struct HttpError {
	/// The HTTP status code
	pub status_code: StatusCode,
	/// The error message
	pub message: String,
}


impl HttpError {
	pub fn from_status(status_code: StatusCode) -> Self {
		Self {
			status_code,
			message: Default::default(),
		}
	}

	/// Creates a new [`AppError`] with the given status code and message.
	pub fn new(status_code: StatusCode, message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
			status_code,
		}
	}
	pub fn not_found() -> Self { Self::from_status(StatusCode::NOT_FOUND) }

	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::new(StatusCode::BAD_REQUEST, message)
	}
	pub fn internal_error(message: impl Into<String>) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
	}

	/// Unwraps the `BevyError` into a `HttpError` if thats what it is,
	/// otherwise logs the error and returns an opaque internal server error
	/// in release builds.
	pub fn from_opaque(error: impl Into<BevyError>) -> Self {
		let error = error.into();
		if let Some(inner) = error.downcast_ref::<HttpError>() {
			// If the error is already an HttpError, return it directly
			inner.clone()
		} else {
			// we are about to lose the internal bevy message, so log it
			// and return an opaque error
			error!("Internal BevyError: {}", error);
			#[cfg(debug_assertions)]
			{
				HttpError::internal_error(format!(
					"Internal Error (debug): {}",
					error
				))
			}
			#[cfg(not(debug_assertions))]
			{
				HttpError::internal_error(format!("Internal Error"))
			}
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
	fn from(err: RunSystemError) -> HttpError { Self::from_opaque(err) }
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

#[cfg(feature = "serde")]
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
				"text/plain",
			)
		}
	}
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
impl axum::response::IntoResponse for HttpError {
	fn into_response(self) -> axum::response::Response {
		(self.status_code, self.message).into_response()
	}
}
