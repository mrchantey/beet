#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
// use beet_agent::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

#[beet_core::test]
async fn works() {
	dotenv::dotenv().ok();
	let mut res = Request::post("https://api.openai.com/v1/chat/completions")
		.with_auth_bearer(&env_ext::var("OPENAI_API_KEY").unwrap())
		.with_json_body(&serde_json::json! {{
			// "model": "gpt-5",
			"model": "gpt-4o-mini",
			"stream":true,
			"messages": [
				{
					"role": "user",
					"content": "Write a 200 word essay about similarities and differences between humans and ai"
				}
			],
		}})
		.unwrap()
		.send()
		.await
		.unwrap()
		.event_source()
		.await
		.unwrap();

	let done = |msg: &str| {
		println!("");
		println!("final message: {msg}");
	};

	while let Some(Ok(text)) = res.next().await {
		use serde_json::Value;


		let Ok(json) = serde_json::from_str::<Value>(&text.data) else {
			done(&text.data);
			break;
		};
		if let Some(data) = json["choices"][0]["delta"]["content"].as_str() {
			print!("{}", data);
		} else {
			done(&text.data);
			break;
		}
	}
	println!("done");

	// 	.text()
	// 	.await
	// 	.unwrap();
	// println!("text: {res}");
}
