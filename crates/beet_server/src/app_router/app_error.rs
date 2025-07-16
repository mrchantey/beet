use beet_router::as_beet::SystemInput;
use bevy::ecs::system::RegisteredSystemError;
use bevy::ecs::system::RunSystemError;
use bevy::prelude::*;
use http::StatusCode;
use tracing::error;


pub type AppResult<T> = std::result::Result<T, AppError>;


/// A http error returned by the app router.
/// The message *will be* returned to the client so ensure that no sensitive information is included.
#[derive(Debug, Clone)]
pub struct AppError {
	/// The error message
	pub message: String,
	/// The HTTP status code
	pub status_code: StatusCode,
}
impl std::fmt::Display for AppError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}: {}", self.status_code, self.message)
	}
}

// impl IntoResponse for BevyError {
// 	fn into_response(self) -> Response { AppError::from(self).into_response() }
// }

impl From<BevyError> for AppError {
	fn from(err: BevyError) -> AppError {
		if let Some(inner) = err.downcast_ref::<AppError>() {
			// If the error is already an AppError, return it directly
			inner.clone()
		} else {
			// Otherwise, convert it to an AppError with internal server error status
			AppError::internal_error(format!("Internal BevyError: {}", err))
		}
	}
}
impl From<RunSystemError> for AppError {
	fn from(run_system_error: RunSystemError) -> AppError {
		error!("RunSystemError: {}", run_system_error);
		// dont leak message to the client
		AppError::internal_error(format!("Internal RunSystemError"))
	}
}
impl<T: SystemInput> From<RegisteredSystemError<T>> for AppError {
	fn from(registered_system_error: RegisteredSystemError<T>) -> AppError {
		error!("RegisteredSystemError: {}", registered_system_error);
		// dont leak message to the client
		AppError::internal_error(format!("Internal RegisteredSystemError"))
	}
}

impl std::error::Error for AppError {}

impl AppError {
	/// Creates a new [`AppError`] with the given status code and message.
	pub fn new(status_code: StatusCode, message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
			status_code,
		}
	}
	pub fn not_found(message: impl Into<String>) -> Self {
		Self::new(StatusCode::NOT_FOUND, message)
	}

	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::new(StatusCode::BAD_REQUEST, message)
	}
	pub fn internal_error(message: impl Into<String>) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
	}
}

impl beet_core::prelude::IntoResponse for AppError {
	fn into_response(self) -> beet_core::prelude::Response {
		beet_core::prelude::Response::from_status_body(
			self.status_code,
			self.message.as_bytes(),
		)
	}
}
