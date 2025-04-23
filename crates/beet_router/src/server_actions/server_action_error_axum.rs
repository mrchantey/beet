use super::ActionError;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use serde::Serialize;


impl<E> IntoResponse for ActionError<E>
where
	E: Serialize,
{
	fn into_response(self) -> Response {
		(
			StatusCode::from_u16(self.status.into())
				.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
			Json(self.error),
		)
			.into_response()
	}
}
