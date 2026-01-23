//! OpenAI provider supporting the OpenAI API.
//!
//! OpenAI provides cloud-based LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;

impl OpenAIProvider {
	/// GPT-5 Nano - smallest and fastest model.
	pub const GPT_5_NANO: &str = "gpt-5-nano";
	/// GPT-5 Mini - balanced speed and capability.
	pub const GPT_5_MINI: &str = "gpt-5-mini";
	/// GPT-5.2 - most capable model.
	pub const GPT_5_2: &str = "gpt-5.2";

	/// OpenAI Responses API URL.
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
		let api_key = env_ext::var("OPENAI_API_KEY")?;
		Ok(Self {
			inner: OpenResponsesProvider::new(Self::RESPONSES_URL)
				.with_auth(api_key),
		})
	}
}

impl ModelProvider for OpenAIProvider {
	fn provider_slug(&self) -> &'static str { "openai" }

	fn default_small_model(&self) -> &'static str { Self::GPT_5_NANO }
	fn default_tool_model(&self) -> &'static str { Self::GPT_5_MINI }
	fn default_large_model(&self) -> &'static str { Self::GPT_5_2 }

	fn send(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<openresponses::ResponseBody>> {
		Box::pin(self.inner.send(request))
	}

	fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>> {
		Box::pin(self.inner.stream(request))
	}
}
