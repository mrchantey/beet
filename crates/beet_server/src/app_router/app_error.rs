use axum::response::IntoResponse;
use axum::response::Response;
use bevy::ecs::system::RunSystemError;
use http::StatusCode;


pub type AppResult<T> = std::result::Result<T, AppError>;

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
		AppError::internal_error(format!(
			"Failed to run system: {}",
			run_system_error.to_string()
		))
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

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		(self.status_code, self.message).into_response()
	}
}
