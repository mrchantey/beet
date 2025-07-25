#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::*;
	use sweet::prelude::*;

	const HTTPBIN: &str = "https://httpbin.org";

	#[sweet::test]
	// #[ignore = "flaky example.com"]
	async fn works() {
		Request::get("https://example.com")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect()
			.to_be(200);
	}

	#[sweet::test]
	#[ignore = "flaky httpbin"]
	async fn get_works() {
		Request::get(format!("{HTTPBIN}/get"))
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect()
			.to_be(200);
	}

	#[sweet::test]
	#[ignore = "flaky httpbin"]
	async fn post_json_works() {
		Request::post(format!("{HTTPBIN}/post"))
			.with_json_body(&serde_json::json!({"foo": "bar"}))
			.unwrap()
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect()
			.to_be(200);
	}

	#[sweet::test]
	#[ignore = "flaky httpbin"]
	async fn custom_header_works() {
		Request::get(format!("{HTTPBIN}/headers"))
			.with_header("X-Foo", "Bar")
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect()
			.to_be(200);
	}

	#[sweet::test]
	#[ignore = "flaky httpbin"]
	async fn put_and_delete_work() {
		Request::get(format!("{HTTPBIN}/put"))
			.with_method(HttpMethod::Put)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect()
			.to_be(200);

		Request::get(format!("{HTTPBIN}/delete"))
			.with_method(HttpMethod::Delete)
			.send()
			.await
			.unwrap()
			.xmap(|res| res.status())
			.xpect()
			.to_be(200);
	}

	#[sweet::test]
	#[ignore = "flaky httpbin"]
	async fn body_raw_works() {
		Request::get(format!("{HTTPBIN}/post"))
			.with_method(HttpMethod::Post)
			.with_body(b"rawbytes".to_vec())
			.send()
			.await
			.unwrap()
			.text()
			.unwrap()
			.xpect()
			.to_contain("rawbytes");
	}


	#[test]
	#[ignore = "flaky httpbin"]

	fn query_params() {
		// #[derive(Serialize)]
		// struct Foo{
		Request::get(format!("{HTTPBIN}/get"))
			.parse_query_param("foo", &(1, 2))
			.xpect()
			.to_be_err();
	}

	#[sweet::test]
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
			.unwrap()
			.xpect()
			.to_contain("baz");
	}

	// #[test]
	// fn bad_url_fails() { Request::get("/foobar"); }
	#[test]
	#[should_panic]
	fn invalid_header_fails() {
		Request::get("http://localhost")
			.with_header("bad\nheader", "val");
	}
}
