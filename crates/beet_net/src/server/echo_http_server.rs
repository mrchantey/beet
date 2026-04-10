//! Local echo HTTP server for testing.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;
use bytes::Bytes;

/// Structured response from the echo server.
///
/// Contains the request details echoed back:
/// method, path segments, headers, query parameters, and body text.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EchoResponse {
	/// The HTTP method used, ie `GET`, `POST`.
	pub method: HttpMethod,
	/// Path segments from the request URL.
	pub path: Vec<String>,
	/// Request headers as a multimap.
	pub headers: MultiMap<String, String>,
	/// Query parameters as a multimap.
	pub query: MultiMap<String, String>,
	/// The request body as text.
	pub body: String,
}

/// Test event payload for SSE endpoint.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SseTestEvent {
	/// The event index.
	pub index: u32,
	/// The event message.
	pub msg: String,
}

/// A local echo HTTP server for integration tests.
///
/// Returns request details as a JSON response body, including method, path,
/// headers, body, and query params. Also supports SSE and streaming JSON endpoints.
pub struct EchoHttpServer {
	/// The base URL of the running server, ie `http://127.0.0.1:38401`.
	url: Url,
}

impl EchoHttpServer {
	/// The base [`Url`] of the running server.
	pub fn url(&self) -> &Url { &self.url }

	/// Starts a new echo HTTP server on a background thread.
	///
	/// Use [`url()`](Self::url) to make requests against the running server.
	pub async fn new() -> Self {
		let server = HttpServer::new_test(start_mini_http_server_with_tcp);
		let url = Url::parse(&server.0.local_url());
		std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((server, exchange_handler(echo_request)))
				.run();
		});
		time_ext::sleep_millis(100).await;
		Self { url }
	}
}

/// Routes requests based on the first path segment.
fn echo_request(req: ToolContext<Request>) -> Response {
	let req = req.take();
	match req.path().first().map(|seg| seg.as_str()) {
		Some("sse") => sse_response(&req),
		Some("stream") => stream_response(&req),
		_ => standard_echo_response(req),
	}
}

/// Builds a JSON response echoing the request's method, path, headers, body, and query params.
fn standard_echo_response(req: Request) -> Response {
	let method = *req.method();
	let path = req.path().to_vec();

	let mut headers = MultiMap::new();
	for (key, values) in req.headers.iter_all() {
		for value in values {
			headers.insert(key.clone(), value.clone());
		}
	}

	let mut query = MultiMap::new();
	for (key, values) in req.params().iter_all() {
		for value in values {
			query.insert(key.clone(), value.clone());
		}
	}

	// Extract body last since `try_into_bytes` consumes it
	let body = req
		.body
		.try_into_bytes()
		.map(|bytes| String::from_utf8_lossy(bytes.as_ref()).into_owned())
		.unwrap_or_default();

	let echo = EchoResponse {
		method,
		path,
		headers,
		query,
		body,
	};

	let json = serde_json::to_string(&echo).unwrap();
	Response::ok_body(json, MediaType::Json)
}

/// Returns a Server-Sent Events stream.
/// Accepts an optional `count` query param (defaults to 3).
fn sse_response(req: &Request) -> Response {
	let count: u32 = req
		.get_param("count")
		.and_then(|val| val.parse().ok())
		.unwrap_or(3);

	let stream = futures::stream::iter((0..count).map(|idx| {
		let event = SseTestEvent {
			index: idx,
			msg: "hello".into(),
		};
		let data = serde_json::to_string(&event).unwrap();
		let formatted = format!("event: message\ndata: {}\n\n", data);
		Ok(Bytes::from(formatted))
	}));

	Response::ok()
		.with_content_type(MediaType::EventStream)
		.with_body(Body::stream(stream))
}

/// Returns newline-delimited JSON objects as a stream.
/// Path format: `/stream/{count}` where count defaults to 3.
fn stream_response(req: &Request) -> Response {
	let count: u32 = req
		.path()
		.get(1)
		.and_then(|val| val.parse().ok())
		.unwrap_or(3);

	let stream = futures::stream::iter((0..count).map(|idx| {
		let json = serde_json::json!({"index": idx, "data": "stream-chunk"});
		let line = format!("{}\n", json);
		Ok(Bytes::from(line))
	}));

	Response::ok()
		.with_content_type(MediaType::Json)
		.with_body(Body::stream(stream))
}


#[cfg(test)]
#[cfg(all(
	feature = "server",
	feature = "ureq",
	feature = "json",
	not(target_arch = "wasm32")
))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Helper to send a request and deserialize the echo response.
	async fn echo(request: Request) -> EchoResponse {
		request
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap()
			.json::<EchoResponse>()
			.await
			.unwrap()
	}

	#[beet_core::test]
	async fn echo_get() {
		let server = EchoHttpServer::new().await;
		let resp = echo(Request::get(server.url().clone().push("get"))).await;
		resp.method.xpect_eq(HttpMethod::Get);
		resp.path.xpect_eq(vec!["get".to_string()]);
	}

	#[beet_core::test]
	async fn echo_post_with_body() {
		let server = EchoHttpServer::new().await;
		let resp = echo(
			Request::post(server.url().clone().push("post"))
				.with_body("hello world"),
		)
		.await;
		resp.method.xpect_eq(HttpMethod::Post);
		resp.body.xpect_eq("hello world");
	}

	#[beet_core::test]
	async fn echo_custom_header() {
		let server = EchoHttpServer::new().await;
		let resp = echo(
			Request::get(server.url().clone().push("headers"))
				.with_header_raw("x-foo", "Bar"),
		)
		.await;
		resp.headers.get("x-foo").unwrap().xpect_eq("Bar");
	}

	#[beet_core::test]
	async fn echo_query_params() {
		let server = EchoHttpServer::new().await;
		let resp = echo(
			Request::get(server.url().clone().push("search"))
				.with_param("foo", "bar")
				.with_param("baz", "42"),
		)
		.await;
		resp.query.get("foo").unwrap().xpect_eq("bar");
		resp.query.get("baz").unwrap().xpect_eq("42");
	}

	#[beet_core::test]
	async fn echo_sse() {
		let server = EchoHttpServer::new().await;
		let resp = Request::get(server.url().clone().push("sse"))
			.with_param("count", "2")
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap();
		let text = resp.text().await.unwrap();
		text.xref().xpect_contains("event: message");
		text.xref().xpect_contains("\"index\":0");
		text.xref().xpect_contains("\"index\":1");
	}
}
