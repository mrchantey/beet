#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
use base64::prelude::*;
use beet_net::prelude::*;
use beet_core::prelude::*;

#[sweet::test]
async fn works() {
	dotenv::dotenv().ok();
	//https://ai.google.dev/api/generate-content#method:-models.generatecontent
	let res = Request::post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image-preview:generateContent")
		.with_header("x-goog-api-key", &std::env::var("GEMINI_API_KEY").unwrap())
		.with_json_body(&serde_json::json! {{
			"contents": [{
				"role": "user",
				"parts": [
					{
						"text": "Create a picture of a nano banana dish in a fancy restaurant with a Gemini theme"
					}
				]
			}]
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

	FsExt::write("dump.txt", res.xfmt()).unwrap();


	// Extract the base64 image string (response expected to contain a top-level "data" field)
	let b64 =
		&res["candidates"][0]["content"]["parts"][1]["inlineData"]["data"]
			.as_str()
			.unwrap();

	// Decode base64 to bytes
	let bytes = BASE64_STANDARD.decode(b64).unwrap();

	let path = AbsPathBuf::new_workspace_rel(".cache/image.png").unwrap();

	FsExt::write(path, bytes).unwrap();
}
