use axum::extract::Request;
use axum::handler::HandlerWithoutStateExt;
use axum::response::IntoResponse;
use http::StatusCode;
use std::convert::Infallible;
use tower::Service;
use tower_http::services::ServeDir;

/// Serve files as a fallback, if none are found return 404.
/// TODO its bad practice to serve files from lambda,
/// this should use a bucket instead
pub fn file_and_error_handler(
	file_dir: &str,
) -> impl Service<
	Request,
	Error = Infallible,
	Future = impl Send + 'static,
	Response = impl IntoResponse,
> + Clone
+ Send
+ 'static {
	async fn handle_404() -> (StatusCode, &'static str) {
		(StatusCode::NOT_FOUND, "File Not found")
	}

	let serve_dir =
		ServeDir::new(file_dir).not_found_service(handle_404.into_service());

	serve_dir
}
