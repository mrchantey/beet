//! HTTP request sending functionality.
//!
//! This module provides the [`RequestClientExt`] extension trait that adds
//! a `send()` method to [`Request`] for executing HTTP requests.
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

/// Extension trait for sending HTTP requests.
///
/// This trait provides a unified `send()` method that works across platforms,
/// automatically selecting the appropriate HTTP backend based on target and features.
#[extend::ext(name=RequestClientExt)]
pub impl Request {
	/// Sends this request and returns the response.
	///
	/// # Platform Behavior
	///
	/// - **WASM**: Uses `web-sys` fetch API
	/// - **Native + `ureq`**: Uses blocking ureq, wrapped in unblock + async
	/// - **Native + `reqwest`**: Uses async reqwest client
	///
	/// # Errors
	///
	/// Returns an error if the request fails due to network issues,
	/// invalid URLs, or missing TLS features for HTTPS requests.
	#[allow(async_fn_in_trait)]
	async fn send(self) -> Result<Response> {
		#[cfg(target_arch = "wasm32")]
		{
			super::impl_web_sys::send_wasm(self).await
		}
		#[cfg(all(feature = "ureq", not(target_arch = "wasm32")))]
		{
			super::impl_ureq::send_ureq(self).await
		}
		#[cfg(all(
			feature = "reqwest",
			not(feature = "ureq"),
			not(target_arch = "wasm32")
		))]
		{
			super::impl_reqwest::send_reqwest(self).await
		}

		#[cfg(not(any(
			feature = "reqwest",
			feature = "ureq",
			target_arch = "wasm32"
		)))]
		{
			panic!(
				"No HTTP transport available, enable the 'reqwest' or 'ureq' feature for native builds"
			);
		}
	}
}



#[cfg(any(
	all(feature = "ureq", feature = "native-tls"),
	all(feature = "reqwest", feature = "native-tls"),
	target_arch = "wasm32"
))]
#[cfg(test)]
mod test_request {
	use crate::prelude::*;
	use beet_core::prelude::*;

	const HTTPBIN: &str = "https://postman-echo.com";
	// const HTTPBIN: &str = "https://httpbin.org";
	// TODO spin up our own server for tests
	#[cfg_attr(feature = "reqwest", beet_core::test(tokio))]
	#[cfg_attr(not(feature = "reqwest"), beet_core::test)]
	// #[ignore = "flaky example.com"]
	async fn works() {
		Request::get("https://example.com")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn get_works() {
		Request::get(format!("{HTTPBIN}/get"))
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::Ok);
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
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	#[ignore = "flaky httpbin"]
	async fn custom_header_works() {
		Request::get(format!("{HTTPBIN}/headers"))
			.with_header("X-Foo", "Bar")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::Ok);
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
			.xpect_eq(StatusCode::Ok);

		Request::get(format!("{HTTPBIN}/delete"))
			.with_method(HttpMethod::Delete)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(StatusCode::Ok);
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

		res1.unwrap().status().xpect_eq(StatusCode::Ok);
		res2.unwrap().status().xpect_eq(StatusCode::Ok);
		res3.unwrap().status().xpect_eq(StatusCode::Ok);

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

	#[test]
	#[should_panic]
	fn invalid_header_fails() {
		Request::get("http://localhost").with_header("bad\nheader", "val");
	}
}


#[cfg(test)]
#[cfg(any(feature = "reqwest", feature = "ureq", target_arch = "wasm32"))]
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
