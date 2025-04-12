use axum::extract::Request;
use axum::handler::HandlerWithoutStateExt;
use axum::response::IntoResponse;
use http::HeaderValue;
use http::StatusCode;
use http::header::CONTENT_TYPE;
use std::convert::Infallible;
use std::path::Path;
use tower::Service;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

/// Serve files as a fallback, if none are found return 404.
/// TODO its bad practice to serve files from lambda,
/// this should use a bucket instead
pub fn file_and_error_handler(
	file_dir: &Path,
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

	let serve_dir = ServeDir::new(file_dir)
		// .append_index_html_on_directories(false)
		.not_found_service(handle_404.into_service());


	// ServeDir will append a trailing slash which screws up our
	// route matching, so we assume any route without a file extension
	// is a directory and append index.html
	tower::ServiceBuilder::new()
		.layer_fn(|mut svc: ServeDir<_>| {
			tower::service_fn(move |req: Request| {
				let mut req = req;
				let uri = req.uri().to_string();

				if !uri.contains('.') && !uri.ends_with('/') {
					let new_uri = format!("{}/index.html", uri);
					*req.uri_mut() = new_uri.parse().unwrap();
				}

				svc.call(req)
			})
		})
		.service(serve_dir)
}
