#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use base64::prelude::*;
use beet_net::prelude::*;
use beet_core::prelude::*;

#[sweet::test]
async fn works() {
	dotenv::dotenv().ok();
	let res = Request::post("https://api.openai.com/v1/images/generations")
		.with_auth_bearer(&std::env::var("OPENAI_API_KEY").unwrap())
		.with_json_body(&serde_json::json! {{
			"model": "gpt-image-1",
			"prompt": "An Arduino Alvic with a phone mounted on top horizontally, on the phone cute happy robot eyes like eve from Wall-E
			the robot is greeting a user. Style of the image is a cartoon",
			"n": 1,
			"size": "1024x1024"
		}})
		.unwrap()
		.send()
		.await
		.unwrap()
		.into_result()
		.await
		.unwrap()
		.json::<serde_json::Value>()
		.await
		.unwrap();
	// Extract the base64 image string
	let b64 = &res["data"][0]["b64_json"].as_str().unwrap();

	// Decode base64 to bytes
	let bytes = BASE64_STANDARD
		.decode(b64)
		.expect("Failed to decode base64");

	let path = AbsPathBuf::new_workspace_rel(".cache/otter.png").unwrap();

	fs_ext::write(path, bytes).unwrap();
}
