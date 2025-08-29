#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// use beet_agent::prelude::*;
use beet_core::prelude::*;
// use beet_utils::prelude::*;
// use sweet::prelude::*;

#[sweet::test]
async fn works() {
	dotenv::dotenv().ok();
	let mut res = Request::post("https://api.openai.com/v1/chat/completions")
		.with_auth_bearer(&std::env::var("OPENAI_API_KEY").unwrap())
		.with_json_body(&serde_json::json! {{
			"model": "gpt-4o-mini",
			"stream":true,
			"messages": [
				{
					"role": "user",
					"content": "Write a one-sentence bedtime story about a unicorn."
				}
			],
		}})
		.unwrap()
		.send()
		.await
		.unwrap()
		.into_result()
		.await
		.unwrap();

	while let Some(text) = res.body.next().await.unwrap() {
		let str = String::from_utf8_lossy(&text);
		println!("chunk: {str}");
	}

	// 	.text()
	// 	.await
	// 	.unwrap();
	// println!("text: {res}");
}
