use axum::response::Response;
use bevy::ecs::system::RunSystemError;
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

impl From<RunSystemError> for AppError {
	fn from(run_system_error: RunSystemError) -> AppError {
		error!("Run System Error: {}", run_system_error);
		// dont leak message to the client
		AppError::internal_error(format!("Internal Run System Error"))
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
	pub fn bad_request(message: impl Into<String>) -> Self {
		Self::new(StatusCode::BAD_REQUEST, message)
	}
	pub fn internal_error(message: impl Into<String>) -> Self {
		Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
	}
}

impl axum::response::IntoResponse for AppError {
	fn into_response(self) -> Response {
		(self.status_code, self.message).into_response()
	}
}


impl beet_core::http_resources::IntoResponse for AppError {
	fn into_response(self) -> beet_core::http_resources::Response {
		beet_core::http_resources::Response::from_status_body(
			self.status_code,
			self.message.as_bytes(),
		)
	}
}
