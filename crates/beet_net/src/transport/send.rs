use crate::prelude::*;
use beet_core::prelude::*;

impl Request {
	pub async fn send(self) -> Result<Response> {
		#[cfg(target_arch = "wasm32")]
		{
			super::impl_web_sys::send_wasm(self).await
		}
		#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
		{
			super::impl_reqwest::send_reqwest(self).await
		}
		#[cfg(not(any(feature = "reqwest", target_arch = "wasm32")))]
		{
			panic!(
				"No HTTP transport available, enable the 'reqwest' feature for native builds"
			);
		}
	}
}



#[cfg(any(feature = "reqwest", target_arch = "wasm32"))]
#[cfg(test)]
mod test_request {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	const HTTPBIN: &str = "https://postman-echo.com";
	// const HTTPBIN: &str = "https://httpbin.org";

	#[sweet::test(tokio)]
	// #[ignore = "flaky example.com"]
	async fn works() {
		Request::get("https://example.com")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(200);
	}

	#[sweet::test(tokio)]
	#[ignore = "flaky httpbin"]
	async fn get_works() {
		Request::get(format!("{HTTPBIN}/get"))
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(200);
	}

	#[sweet::test(tokio)]
	#[ignore = "flaky httpbin"]
	async fn post_json_works() {
		Request::post(format!("{HTTPBIN}/post"))
			.with_json_body(&serde_json::json!({"foo": "bar"}))
			.unwrap()
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(200);
	}

	#[sweet::test(tokio)]
	#[ignore = "flaky httpbin"]
	async fn custom_header_works() {
		Request::get(format!("{HTTPBIN}/headers"))
			.with_header("X-Foo", "Bar")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(200);
	}

	#[sweet::test(tokio)]
	#[ignore = "flaky httpbin"]
	async fn put_and_delete_work() {
		Request::get(format!("{HTTPBIN}/put"))
			.with_method(HttpMethod::Put)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(200);

		Request::get(format!("{HTTPBIN}/delete"))
			.with_method(HttpMethod::Delete)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect_eq(200);
	}

	#[sweet::test(tokio)]
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

	#[sweet::test(tokio)]
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


	#[test]
	#[ignore = "flaky httpbin"]
	fn query_params() {
		// #[derive(Serialize)]
		// struct Foo{
		Request::get(format!("{HTTPBIN}/get"))
			.parse_query_param("foo", &(1, 2))
			.xpect_err();
	}

	#[sweet::test(tokio)]
	#[ignore = "flaky httpbin"]
	async fn query_params_work() {
		Request::get(format!("{HTTPBIN}/get"))
			.with_query_param("foo", "bar")
			.unwrap()
			.with_query_param("baz", "qux")
			.unwrap()
			.send()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("baz");
	}

	// #[test]
	// fn bad_url_fails() { Request::get("/foobar"); }
	#[test]
	#[should_panic]
	#[cfg(not(target_arch = "wasm32"))] // sweet panic catch broken :(
	fn invalid_header_fails() {
		Request::get("http://localhost").with_header("bad\nheader", "val");
	}
}


#[cfg(test)]
#[cfg(any(feature = "reqwest", target_arch = "wasm32"))]
mod test_response {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;


	// const HTTPBIN: &str = "https://httpbin.org";
	const HTTPBIN: &str = "https://httpbin.dev";

	#[sweet::test(tokio)]
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
	#[sweet::test(tokio)]
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
