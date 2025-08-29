use axum::extract::Request;
use axum::handler::HandlerWithoutStateExt;
use axum::response::IntoResponse;
use http::StatusCode;
use std::convert::Infallible;
use std::path::Path;
use tower::Service;
use tower_http::services::ServeDir;

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
	async fn handle_404() -> StatusCode { StatusCode::NOT_FOUND }

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
				let mut uri = req.uri().to_string();

				// Extract the last path component
				// Check if the last component has a file extension
				if !uri
					.split('/')
					.last()
					.map(|p| p.contains('.'))
					.unwrap_or(false)
				{
					if uri.ends_with('/') {
						uri.pop();
					}
					uri.push_str("/index.html");
					*req.uri_mut() = uri.parse().unwrap();
				}

				svc.call(req)
			})
		})
		.service(serve_dir)
}
