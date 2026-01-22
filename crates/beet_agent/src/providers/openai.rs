//! OpenAI provider supporting the OpenAI API.
//!
//! OpenAI provides cloud-based LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;
use futures::Stream;


pub mod constants {
	pub const GPT_5_MINI: &str = "gpt-5-mini";
	pub const GPT_5_2: &str = "gpt-5.2";

	pub const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
}

/// An OpenResponses-compatible provider for OpenAI API.
///
/// OpenAI API key must be set via the `OPENAI_API_KEY` environment variable.
/// By default, connects to `https://api.openai.com/v1`.
pub struct OpenAIProvider {
	inner: OpenResponsesProvider,
}

impl Default for OpenAIProvider {
	fn default() -> Self { Self::new().unwrap() }
}

impl OpenAIProvider {
	/// Creates a new provider with the API key from the environment.
	pub fn new() -> Result<Self> {
		let api_key = std::env::var("OPENAI_API_KEY")?;
		Ok(Self {
			inner: OpenResponsesProvider::new(constants::RESPONSES_URL)
				.with_auth(api_key),
		})
	}
}

impl ModelProvider for OpenAIProvider {
	fn default_small_model(&self) -> &'static str { constants::GPT_5_MINI }
	fn default_tool_model(&self) -> &'static str { constants::GPT_5_MINI }
	fn default_large_model(&self) -> &'static str { constants::GPT_5_2 }

	fn send(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<openresponses::ResponseBody>> {
		self.inner.send(request)
	}

	fn stream(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<
		Output = Result<
			impl Stream<Item = Result<openresponses::StreamingEvent>>,
		>,
	> {
		self.inner.stream(request)
	}
}
