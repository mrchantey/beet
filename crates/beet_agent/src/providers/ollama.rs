use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::exports::eventsource_stream::EventStream;
use beet_net::prelude::*;
use std::borrow::Cow;


pub mod models {
	pub const QWEN_3_ABLITERATED_14B: &str = "huihui_ai/qwen3-abliterated:14b";
	pub const FUNCTION_GEMMA_270M_IT: &str = "functiongemma:270m-it-fp16";
	pub const QWEN_3_8B: &str = "qwen3:8b";
}

pub struct OllamaProvider {
	/// The full url to the openresponses compatible ollama endpoint,
	/// defaults to `http://localhost:11434/v1/responses`
	url: Cow<'static, str>,
}

impl Default for OllamaProvider {
	fn default() -> Self {
		Self {
			url: "http://localhost:11434/v1/responses".into(),
		}
	}
}

impl ModelProvider for OllamaProvider {
	fn default_small_model(&self) -> &'static str { models::QWEN_3_8B }
	fn default_tool_model(&self) -> &'static str {
		models::FUNCTION_GEMMA_270M_IT
	}
	fn default_large_model(&self) -> &'static str {
		models::QWEN_3_ABLITERATED_14B
	}

	fn send(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<openresponses::ResponseBody>> {
		async move {
			Request::post(&self.url)
				// .with_auth_bearer(&env_ext::var("OLLAMA_API_KEY")?)
				.with_json_body::<openresponses::RequestBody>(&request)
				.unwrap()
				.send()
				.await?
				.into_result()
				.await?
				.json::<openresponses::ResponseBody>()
				.await
		}
	}
	fn stream(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<EventStream<Body>>> {
		async move {
			if request.stream != Some(true) {
				bevybail!(
					"streaming must be enabled in the request to use the stream method"
				);
			}
			todo!("event stream parsing")
		}
	}
}
