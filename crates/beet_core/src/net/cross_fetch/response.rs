#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::utils::PipelineTarget;
	use sweet::prelude::*;


	const HTTPBIN: &str = "https://httpbin.org";

	#[derive(Debug, PartialEq, serde::Deserialize)]
	struct Res {
		data: Body,
	}
	#[derive(Debug, PartialEq, serde::Deserialize)]
	struct Body {
		foo: String,
	}

	#[sweet::test]
	#[ignore = "flaky httpbin"]
	async fn works() {
		Request::post(format!("{HTTPBIN}/post"))
			.with_body(&serde_json::json!({"foo": "bar"}).to_string())
			.send()
			.await
			.unwrap()
			.json::<serde_json::Value>()
			.unwrap()
			.xmap(|value| value["json"]["foo"].as_str().unwrap().to_string())
			.xpect()
			.to_be("bar");
	}
}
