//! Request sending with scheme-based routing.
//!
//! This module provides the [`Request::send`] method that routes requests
//! based on their URL scheme:
//!
//! - `http` | `https` → HTTP client (ureq, reqwest, or web-sys)
//! - `file` → local filesystem via [`FileClient`]
//! - `data` → inline data URI, decoded to a 200 response
//! - No scheme with authority → HTTP client
//! - No scheme without authority → local filesystem via [`FileClient`]
//! - Other → returns an error
use crate::prelude::*;
use beet_core::prelude::*;

/// Validates that appropriate TLS features are enabled for HTTPS requests.
#[allow(unused)]
pub(super) fn check_https_features(_req: &Request) -> Result {
	#[cfg(not(any(feature = "rustls-tls", feature = "native-tls")))]
	{
		if _req.scheme() == &Scheme::Https {
			beet_core::bevybail!(
				"Please enable either `beet/rustls-tls` or `beet/native-tls` feature to use HTTPS requests."
			);
		}
	}
	Ok(())
}

/// Send a request via the appropriate HTTP backend.
///
/// This is the HTTP-specific send path used by scheme routing.
#[allow(unused)]
async fn send_http(request: Request) -> Result<Response> {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			super::impl_web_sys::send_wasm(request).await
		} else if #[cfg(feature = "ureq")] {
			super::impl_ureq::send_ureq(request).await
		} else if #[cfg(feature = "reqwest")] {
			super::impl_reqwest::send_reqwest(request).await
		} else {
			bevybail!(
				"No HTTP transport available, enable the 'reqwest' or 'ureq' feature for native builds"
			);
		}
	}
}

/// Send a request via the local filesystem [`FileClient`].
#[cfg(feature = "fs")]
async fn send_file(request: Request) -> Result<Response> {
	let url = request.url();
	let path = match url.scheme() {
		// file:// URLs produce absolute paths via path_string()
		Scheme::File => url.path_string(),
		// No scheme — join segments without leading `/` to keep relative
		_ => url.path().join("/"),
	};
	FileClient::new().send(path).await
}

/// Extension trait for sending HTTP requests.
///
/// This trait provides a unified `send()` method that works across platforms,
/// automatically selecting the appropriate backend based on the URL scheme.
impl Request {
	/// Sends this request and returns the response.
	///
	/// # Scheme Routing
	///
	/// | Scheme | Backend |
	/// |--------|---------|
	/// | `http` / `https` | HTTP client (ureq, reqwest, or web-sys) |
	/// | `file` | Local filesystem via [`FileClient`] |
	/// | `data` | Inline data URI decoded to a 200 response |
	/// | None + authority present | HTTP client |
	/// | None + no authority | Local filesystem via [`FileClient`] |
	/// | Other | Returns an error |
	///
	/// # Errors
	///
	/// Returns an error if the request fails due to network issues,
	/// invalid URLs, missing TLS features for HTTPS requests, or
	/// an unsupported scheme.
	pub async fn send(self) -> Result<Response> {
		match self.scheme() {
			Scheme::Http | Scheme::Https => send_http(self).await,
			Scheme::File => {
				cfg_if! {
					if #[cfg(feature = "fs")] {
						send_file(self).await
					} else {
						bevybail!(
							"The 'fs' feature is required for file:// requests"
						);
					}
				}
			}
			Scheme::None => {
				if self.url().authority().is_some() {
					// Authority present without a scheme, assume HTTP
					send_http(self).await
				} else {
					// No authority — treat as a local file path
					cfg_if! {
						if #[cfg(feature = "fs")] {
							send_file(self).await
						} else {
							bevybail!(
								"The 'fs' feature is required for local file requests"
							);
						}
					}
				}
			}
			Scheme::About
				if self.path().first() == Some(&String::from("blank")) =>
			{
				Ok(Response::ok())
			}
			Scheme::Ws | Scheme::Wss => {
				bevybail!(
					"WebSocket schemes are not supported by Request::send, use the sockets module instead"
				);
			}
			Scheme::Data => send_data(self).await,
			Scheme::MailTo
			| Scheme::Tel
			| Scheme::JavaScript
			| Scheme::Blob
			| Scheme::Cid
			| Scheme::About
			| Scheme::Chrome => {
				bevybail!(
					"Non-hierarchical scheme '{}' is not supported by Request::send",
					self.scheme()
				);
			}
			Scheme::Other(scheme) => {
				bevybail!("Unsupported URL scheme: {scheme}");
			}
		}
	}
}

/// Serve an inline data URI as a synthetic 200 response.
///
/// Parses the data URI payload, decodes the body, and sets the `Content-Type`
/// header to the declared media type. The `Accept` header on the request is
/// respected — if the declared media type is not acceptable a 406 response is
/// returned.
async fn send_data(request: Request) -> Result<Response> {
	let mb = MediaBytes::from_url(request.url())?;

	// Content negotiation: check Accept header if present.
	let accepts = request
		.headers()
		.get::<header::Accept>()
		.and_then(|res| res.ok())
		.unwrap_or_default();

	if !accepts.is_empty() && !accepts.contains(mb.media_type()) {
		return Ok(Response::from_status(StatusCode::NOT_ACCEPTABLE));
	}

	Response::ok()
		.with_content_type(mb.media_type().clone())
		.with_body(mb.bytes().to_vec())
		.xok()
}


#[cfg(test)]
mod test_data_scheme {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn data_text_plain() {
		let response = Request::get("data:text/plain,Hello%20World")
			.send()
			.await
			.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response
			.parts
			.headers
			.get::<crate::headers::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Text);
	}

	#[beet_core::test]
	async fn data_html() {
		let response = Request::get("data:text/html,<h1>Hi</h1>")
			.send()
			.await
			.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		let text = response.text().await.unwrap();
		text.xpect_contains("<h1>Hi</h1>");
	}

	#[beet_core::test]
	async fn data_base64() {
		// "Hello" base64-encoded
		let response = Request::get("data:text/plain;base64,SGVsbG8=")
			.send()
			.await
			.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response.text().await.unwrap().xpect_eq("Hello");
	}

	#[beet_core::test]
	async fn data_accept_mismatch_returns_406() {
		let response = Request::get("data:text/html,<h1>Hi</h1>")
			.with_accept(MediaType::Json)
			.send()
			.await
			.unwrap();
		response.status().xpect_eq(StatusCode::NOT_ACCEPTABLE);
	}
}

#[cfg(test)]
#[cfg(feature = "json")]
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod test_request {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	#[ignore = "requires external network and system CA certs"]
	async fn works() {
		Request::get("https://example.com")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn get_works() {
		let server = EchoHttpServer::new().await;
		Request::get(server.url().clone().push("get"))
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn post_json_works() {
		let server = EchoHttpServer::new().await;
		Request::post(server.url().clone().push("post"))
			.with_json_body(&serde_json::json!({"foo": "bar"}))
			.unwrap()
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn custom_header_works() {
		let server = EchoHttpServer::new().await;
		let resp: EchoResponse =
			Request::get(server.url().clone().push("headers"))
				.with_header_raw("X-Foo", "Bar")
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.unwrap()
				.json()
				.await
				.unwrap();
		resp.headers.get("x-foo").unwrap().xpect_contains("Bar");
	}

	#[beet_core::test]
	async fn put_and_delete_work() {
		let server = EchoHttpServer::new().await;
		Request::get(server.url().clone().push("put"))
			.with_method(HttpMethod::Put)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);

		Request::get(server.url().clone().push("delete"))
			.with_method(HttpMethod::Delete)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn body_raw_works() {
		let server = EchoHttpServer::new().await;
		let resp: EchoResponse =
			Request::get(server.url().clone().push("post"))
				.with_method(HttpMethod::Post)
				.with_body(b"rawbytes".to_vec())
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.unwrap()
				.json()
				.await
				.unwrap();
		resp.body.xpect_contains("rawbytes");
	}

	#[beet_core::test]
	async fn body_stream() {
		use bytes::Bytes;

		let server = EchoHttpServer::new().await;
		let resp: EchoResponse =
			Request::post(server.url().clone().push("post"))
				.with_body_stream(bevy::tasks::futures_lite::stream::iter(
					vec![
						Ok(Bytes::from("chunk1")),
						Ok(Bytes::from("chunk2")),
						Ok(Bytes::from("chunk3")),
					],
				))
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.unwrap()
				.json()
				.await
				.unwrap();
		resp.body.contains("chunk1").xpect_true();
		resp.body.contains("chunk2").xpect_true();
		resp.body.contains("chunk3").xpect_true();
	}

	#[beet_core::test]
	#[ignore = "requires external network and system CA certs"]
	async fn concurrent_requests_complete_independently() {
		let start = Instant::now();

		let req1 = Request::get("https://example.com").send();
		let req2 = Request::get("https://example.com").send();
		let req3 = Request::get("https://example.com").send();

		let (res1, res2, res3) = futures::join!(req1, req2, req3);

		res1.unwrap().status().xpect_eq(StatusCode::OK);
		res2.unwrap().status().xpect_eq(StatusCode::OK);
		res3.unwrap().status().xpect_eq(StatusCode::OK);

		// Should complete concurrently in < 3 seconds, not sequentially
		start.elapsed().as_secs().xpect_less_than(3);
	}

	#[test]
	fn query_params() {
		Request::get("http://localhost/get")
			.parse_query_param("foo", &(1, 2))
			.xpect_err();
	}

	#[beet_core::test]
	async fn query_params_work() {
		let server = EchoHttpServer::new().await;
		let resp: EchoResponse = Request::get(server.url().clone().push("get"))
			.with_param("foo", "bar")
			.with_param("baz", "qux")
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap()
			.json()
			.await
			.unwrap();
		resp.query.get("foo").unwrap().xpect_eq("bar");
		resp.query.get("baz").unwrap().xpect_eq("qux");
	}
}


#[cfg(test)]
#[cfg(any(feature = "reqwest", feature = "ureq", target_arch = "wasm32"))]
#[cfg(feature = "json")]
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod test_response {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn post() {
		let server = EchoHttpServer::new().await;
		let original = serde_json::json!({"foo": "bar"});
		let resp: EchoResponse =
			Request::post(server.url().clone().push("post"))
				.with_body(&original.to_string())
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.unwrap()
				.json()
				.await
				.unwrap();
		// The echo server returns the raw body string; verify it contains our JSON
		resp.body.xpect_contains("\"foo\":\"bar\"");
	}

	#[beet_core::test]
	async fn stream() {
		let server = EchoHttpServer::new().await;
		let res = Request::get(server.url().clone().push("stream").push("3"))
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap();

		matches!(res.body, Body::Stream(_)).xpect_true();

		res.text().await.unwrap().len().xpect_greater_than(10);
	}
}


#[cfg(feature = "fs")]
#[cfg(test)]
mod test_file_scheme {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn send_with_file_scheme() {
		let cwd = fs_ext::current_dir().unwrap();
		let cargo_toml = cwd.join("Cargo.toml");
		if !cargo_toml.exists() {
			return;
		}
		let url = format!("file://{}", cargo_toml.display());
		let response = Request::get(url).send().await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response.text().await.unwrap().xpect_contains("[package]");
	}

	#[beet_core::test]
	async fn send_bare_path() {
		let cwd = fs_ext::current_dir().unwrap();
		let cargo_toml = cwd.join("Cargo.toml");
		if !cargo_toml.exists() {
			return;
		}
		// A bare relative path with no scheme or authority
		let response = Request::get("Cargo.toml").send().await.unwrap();
		response.status().xpect_eq(StatusCode::OK);
		response.text().await.unwrap().xpect_contains("[package]");
	}
}
