//! Local echo HTTP server for testing.
use beet_tool::prelude::*;
use crate::prelude::*;

/// A local echo HTTP server for integration tests.
///
/// Mimics the behavior of httpbin/postman-echo by returning request details
/// as a JSON response body, including method, path, headers, body, and query params.
pub struct EchoHttpServer {
	/// The base URL of the running server, ie `http://127.0.0.1:8401`.
	pub url: String,
}

impl EchoHttpServer {
	/// Starts a new echo HTTP server on a background thread.
	///
	/// Use the `url` field to make requests against the running server.
	pub async fn new() -> Self {
		let server = HttpServer::new_test(start_mini_http_server);
		let url = server.0.local_url();
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

/// Builds a JSON response echoing the request's method, path, headers, body, and query params.
fn echo_request(req: FuncToolIn<Request>) -> Response {
	let req = req.take();

	let method = req.method().to_string().to_uppercase();
	let path = req.path_string();

	let mut headers_map = serde_json::Map::new();
	for (key, values) in req.headers.iter_all() {
		if let Some(first) = values.first() {
			headers_map.insert(
				key.clone(),
				serde_json::Value::String(first.clone()),
			);
		}
	}

	let mut query_map = serde_json::Map::new();
	for (key, values) in req.params().iter_all() {
		if let Some(first) = values.first() {
			query_map.insert(
				key.clone(),
				serde_json::Value::String(first.clone()),
			);
		}
	}

	// Extract body last since `try_into_bytes` consumes it
	let body_text = req
		.body
		.try_into_bytes()
		.map(|bytes| String::from_utf8_lossy(bytes.as_ref()).into_owned())
		.unwrap_or_default();

	let json = serde_json::json!({
		"method": method,
		"path": path,
		"headers": serde_json::Value::Object(headers_map),
		"body": body_text,
		"query": serde_json::Value::Object(query_map),
	});

	Response::ok_body(json.to_string(), MediaType::Json)
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

	#[beet_core::test]
	async fn echo_get() {
		let server = super::EchoHttpServer::new().await;
		let response = Request::get(format!("{}/get", server.url))
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap();
		let body = response.text().await.unwrap();
		let json: serde_json::Value = serde_json::from_str(&body).unwrap();
		json["method"].as_str().unwrap().xpect_eq("GET");
		json["path"].as_str().unwrap().xpect_eq("/get");
	}

	#[beet_core::test]
	async fn echo_post_with_body() {
		let server = super::EchoHttpServer::new().await;
		let response = Request::post(format!("{}/post", server.url))
			.with_body("hello world")
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap();
		let body = response.text().await.unwrap();
		let json: serde_json::Value = serde_json::from_str(&body).unwrap();
		json["method"].as_str().unwrap().xpect_eq("POST");
		json["body"].as_str().unwrap().xpect_eq("hello world");
	}

	#[beet_core::test]
	async fn echo_custom_header() {
		let server = super::EchoHttpServer::new().await;
		let response = Request::get(format!("{}/headers", server.url))
			.with_header_raw("x-foo", "Bar")
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap();
		let body = response.text().await.unwrap();
		let json: serde_json::Value = serde_json::from_str(&body).unwrap();
		json["headers"]["x-foo"]
			.as_str()
			.unwrap()
			.xpect_eq("Bar");
	}

	#[beet_core::test]
	async fn echo_query_params() {
		let server = super::EchoHttpServer::new().await;
		let response = Request::get(format!("{}/search", server.url))
			.with_param("foo", "bar")
			.with_param("baz", "42")
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap();
		let body = response.text().await.unwrap();
		let json: serde_json::Value = serde_json::from_str(&body).unwrap();
		json["query"]["foo"].as_str().unwrap().xpect_eq("bar");
		json["query"]["baz"].as_str().unwrap().xpect_eq("42");
	}
}
