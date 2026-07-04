//! OpenAI provider supporting the OpenAI API.
//!
//! OpenAI provides cloud-based LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;

/// An OpenResponses-compatible provider for OpenAI API.
///
/// OpenAI API key must be set via the `OPENAI_API_KEY` environment variable.
pub struct OpenAiProvider;

impl OpenAiProvider {
	pub const AUTH_ENV: &str = "OPENAI_API_KEY";

	/// Provider slug for OpenAI.
	pub const PROVIDER_SLUG: &str = "openai";
	/// GPT-5.4 Nano - smallest and fastest model.
	pub const GPT_5_4_NANO: &str = "gpt-5.4-nano";
	/// GPT-5.4 Mini - balanced speed and capability.
	pub const GPT_5_4_MINI: &str = "gpt-5.4-mini";
	/// GPT-5.5 - most capable model.
	pub const GPT_5_5: &str = "gpt-5.5";

	/// OpenAI Responses API URL.
	pub const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
	/// OpenAI Chat Completions API URL.
	pub const COMPLETIONS_URL: &str =
		"https://api.openai.com/v1/chat/completions";

	/// Returns an [`O11sStreamer`] configured for GPT-5.4 Mini
	/// via the OpenResponses endpoint.
	pub fn gpt_5_4_mini() -> Result<O11sStreamer> {
		O11sStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::GPT_5_4_MINI.into(),
			url: Self::RESPONSES_URL.into(),
			auth: EnvVar::new(Self::AUTH_ENV)?.xsome(),
		})
		.xok()
	}

	/// Returns a [`CompletionsStreamer`] configured for GPT-5.4 Mini
	/// via the Chat Completions endpoint.
	pub fn gpt_5_4_mini_completions() -> Result<CompletionsStreamer> {
		CompletionsStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::GPT_5_4_MINI.into(),
			url: Self::COMPLETIONS_URL.into(),
			auth: EnvVar::new(Self::AUTH_ENV)?.xsome(),
		})
		.xok()
	}
}
