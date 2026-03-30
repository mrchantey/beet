//! OpenAI provider supporting the OpenAI API.
//!
//! OpenAI provides cloud-based LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;


/// An OpenResponses-compatible provider for OpenAI API.
///
/// OpenAI API key must be set via the `OPENAI_API_KEY` environment variable.
/// By default, connects to `https://api.openai.com/v1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiProvider {
	inner: OpenResponsesProvider,
}

impl Default for OpenAiProvider {
	fn default() -> Self { Self::new().unwrap() }
}

impl OpenAiProvider {
	/// Creates a new provider with the API key from the environment.
	pub fn new() -> Result<Self> {
		let api_key = env_ext::var("OPENAI_API_KEY")?;
		Ok(Self {
			inner: OpenResponsesProvider::new(Self::RESPONSES_URL)
				.with_auth(api_key),
		})
	}
}

impl ModelProvider for OpenAiProvider {
	fn box_clone(&self) -> Box<dyn ModelProvider> { Box::new(self.clone()) }

	fn provider_slug(&self) -> &'static str { Self::PROVIDER_SLUG }

	fn default_small_model(&self) -> &'static str { Self::GPT_5_NANO }
	fn default_tool_model(&self) -> &'static str { Self::GPT_5_MINI }
	fn default_large_model(&self) -> &'static str { Self::GPT_5_2 }

	fn send(
		&self,
		request: o11s::RequestBody,
	) -> BoxedFuture<'_, Result<o11s::ResponseBody>> {
		let request = OpenResponsesProvider::inline_text_file_data(request);
		Box::pin(self.inner.send(request))
	}

	fn stream(
		&self,
		request: o11s::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>> {
		let request = OpenResponsesProvider::inline_text_file_data(request);
		Box::pin(self.inner.stream(request))
	}
}
impl OpenAiProvider {
	pub const AUTH_ENV: &str = "OPENAI_API_KEY";

	/// Provider slug for OpenAI.
	pub const PROVIDER_SLUG: &str = "openai";
	/// GPT-5 Nano - smallest and fastest model.
	pub const GPT_5_NANO: &str = "gpt-5-nano";
	/// GPT-5 Mini - balanced speed and capability.
	pub const GPT_5_MINI: &str = "gpt-5-mini";
	/// GPT-5.2 - most capable model.
	pub const GPT_5_2: &str = "gpt-5.2";

	/// OpenAI Responses API URL.
	pub const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
	/// OpenAI Chat Completions API URL.
	pub const COMPLETIONS_URL: &str =
		"https://api.openai.com/v1/chat/completions";

	/// Returns an [`O11sStreamer`] configured for GPT-5 Mini
	/// via the OpenResponses endpoint.
	pub fn gpt_5_mini() -> Result<O11sStreamer> {
		O11sStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::GPT_5_MINI.into(),
			url: Self::RESPONSES_URL.into(),
			auth: env_ext::var(Self::AUTH_ENV)?.xsome(),
		})
		.xok()
	}

	/// Returns a [`CompletionsStreamer`] configured for GPT-5 Mini
	/// via the Chat Completions endpoint.
	pub fn gpt_5_mini_completions() -> Result<CompletionsStreamer> {
		CompletionsStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::GPT_5_MINI.into(),
			url: Self::COMPLETIONS_URL.into(),
			auth: env_ext::var(Self::AUTH_ENV)?.xsome(),
		})
		.xok()
	}
}
