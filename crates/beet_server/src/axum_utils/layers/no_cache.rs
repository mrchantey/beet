use axum::extract::Request;
use axum::http::header;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;

/// Middleware to add no-cache headers to a response
pub async fn no_cache(request: Request, next: Next) -> Response {
	let response = next.run(request).await;
	append_no_cache_headers(response)
}

/// Append no-cache headers to a response
pub fn append_no_cache_headers(val: impl IntoResponse) -> Response {
	let mut response = val.into_response();
	let headers = response.headers_mut();
	headers.insert(
		header::CACHE_CONTROL,
		HeaderValue::from_static("no-cache, no-store, must-revalidate"),
	);
	headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
	headers.insert(header::EXPIRES, HeaderValue::from_static("0"));
	// do something with `response`...

	response
}

