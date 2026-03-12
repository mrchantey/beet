//! Request sending with scheme-based routing.
//!
//! This module provides the [`Request::send`] method that routes requests
//! based on their URL scheme:
//!
//! - `http` | `https` → HTTP client (ureq, reqwest, or web-sys)
//! - `file` → local filesystem via [`FileClient`]
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
	#[cfg(target_arch = "wasm32")]
	{
		super::impl_web_sys::send_wasm(request).await
	}
	#[cfg(all(feature = "ureq", not(target_arch = "wasm32")))]
	{
		super::impl_ureq::send_ureq(request).await
	}
	#[cfg(all(
		feature = "reqwest",
		not(feature = "ureq"),
		not(target_arch = "wasm32")
	))]
	{
		super::impl_reqwest::send_reqwest(request).await
	}

	#[cfg(not(any(
		feature = "reqwest",
		feature = "ureq",
		target_arch = "wasm32"
	)))]
	{
		bevybail!(
			"No HTTP transport available, enable the 'reqwest' or 'ureq' feature for native builds"
		);
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
				#[cfg(feature = "fs")]
				{
					send_file(self).await
				}
				#[cfg(not(feature = "fs"))]
				{
					bevybail!(
						"The 'fs' feature is required for file:// requests"
					);
				}
			}
			Scheme::None => {
				if self.url().authority().is_some() {
					// Authority present without a scheme, assume HTTP
					send_http(self).await
				} else {
					// No authority — treat as a local file path
					#[cfg(feature = "fs")]
					{
						send_file(self).await
					}
					#[cfg(not(feature = "fs"))]
					{
						bevybail!(
							"The 'fs' feature is required for local file requests"
						);
					}
				}
			}
			Scheme::Ws | Scheme::Wss => {
				bevybail!(
					"WebSocket schemes are not supported by Request::send, use the sockets module instead"
				);
			}
			Scheme::Data
			| Scheme::MailTo
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



#[cfg(any(
	all(feature = "ureq", feature = "native-tls"),
	all(feature = "reqwest", feature = "native-tls"),
	target_arch = "wasm32"
))]
#[cfg(test)]
#[cfg(feature = "json")]
mod test_request {
	use crate::prelude::*;
	use beet_core::prelude::*;

	const HTTPBIN: &str = "https://postman-echo.com";
	// const HTTPBIN: &str = "https://httpbin.org";
	// TODO spin up our own server for tests
	#[cfg_attr(feature = "reqwest", beet_core::test(tokio))]
	#[cfg_attr(not(feature = "reqwest"), beet_core::test)]
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
	#[ignore = "flaky httpbin"]
	async fn get_works() {
		Request::get(format!("{HTTPBIN}/get"))
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn post_json_works() {
		Request::post(format!("{HTTPBIN}/post"))
			.with_json_body(&serde_json::json!({"foo": "bar"}))
			.unwrap()
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn custom_header_works() {
		Request::get(format!("{HTTPBIN}/headers"))
			.with_header_raw("X-Foo", "Bar")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn put_and_delete_work() {
		Request::get(format!("{HTTPBIN}/put"))
			.with_method(HttpMethod::Put)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);

		Request::get(format!("{HTTPBIN}/delete"))
			.with_method(HttpMethod::Delete)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn body_raw_works() {
		Request::get(format!("{HTTPBIN}/post"))
			.with_method(HttpMethod::Post)
			.with_body(b"rawbytes".to_vec())
			.send()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("rawbytes");
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn body_stream() {
		use bytes::Bytes;

		Request::post(format!("{HTTPBIN}/post"))
			.with_body_stream(bevy::tasks::futures_lite::stream::iter(vec![
				Ok(Bytes::from("chunk1")),
				Ok(Bytes::from("chunk2")),
				Ok(Bytes::from("chunk3")),
			]))
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xmap(|text| {
				// cross_log!("Response text: {}", text);
				// The response should contain all our chunks
				text.contains("chunk1")
					&& text.contains("chunk2")
					&& text.contains("chunk3")
			})
			.xpect_true();
	}


	#[cfg_attr(feature = "reqwest", beet_core::test(tokio))]
	#[cfg_attr(not(feature = "reqwest"), beet_core::test)]
	#[ignore = "requires external network and system CA certs"]
	async fn concurrent_requests_complete_independently() {
		// This test verifies that multiple requests can run concurrently
		// without blocking each other. Make 3 concurrent requests - if they're
		// properly async, they'll complete concurrently (fast). If blocking,
		// they'd complete sequentially (slow).
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
	#[ignore = "flaky httpbin"]
	fn query_params() {
		// #[derive(Serialize)]
		// struct Foo{
		Request::get(format!("{HTTPBIN}/get"))
			.parse_query_param("foo", &(1, 2))
			.xpect_err();
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn query_params_work() {
		Request::get(format!("{HTTPBIN}/get"))
			.with_param("foo", "bar")
			.with_param("baz", "qux")
			.send()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("baz");
	}
}


#[cfg(test)]
#[cfg(any(feature = "reqwest", feature = "ureq", target_arch = "wasm32"))]
#[cfg(feature = "json")]
mod test_response {
	use crate::prelude::*;
	use beet_core::prelude::*;

	// const HTTPBIN: &str = "https://httpbin.org";
	const HTTPBIN: &str = "https://httpbin.dev";

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn post() {
		Request::post(format!("{HTTPBIN}/post"))
			.with_body(&serde_json::json!({"foo": "bar"}).to_string())
			.send()
			.await
			.unwrap()
			.json::<serde_json::Value>()
			.await
			.unwrap()
			.xmap(|value| value["json"]["foo"].as_str().unwrap().to_string())
			.xpect_eq("bar");
	}
	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn stream() {
		let res = Request::get(format!("{HTTPBIN}/stream/3"))
			.send()
			.await
			.unwrap();

		matches!(res.body, Body::Stream(_)).xpect_true();

		res.text()
			.await
			.unwrap()
			.len()
			.xpect_greater_than(200)
			.xpect_less_than(1000);
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
