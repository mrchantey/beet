//! Ollama provider supporting the OpenResponses API.
//!
//! Ollama provides local LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;

/// An OpenResponses-compatible provider for local Ollama inference.
///
/// Ollama must be running locally with OpenResponses API support enabled.
/// By default, connects to `http://localhost:11434/v1/responses`.
pub struct OllamaProvider;

impl OllamaProvider {
	pub const PROVIDER_SLUG: &str = "ollama";
	/// Qwen 3 Abliterated 14B - large uncensored model.
	pub const QWEN_3_5_9B_ABLITERATED: &str =
		"huihui_ai/qwen3.5-abliterated:9b";
	/// Function Gemma 270M IT - small function calling model.
	pub const FUNCTION_GEMMA_270M_IT: &str = "functiongemma:270m-it-fp16";
	/// Qwen - balanced model.
	pub const QWEN_3_5_9B: &str = "qwen3.5:9b";

	/// Default OpenResponses URL for local Ollama.
	pub const RESPONSES_URL: &str = "http://localhost:11434/v1/responses";
	/// Default Chat Completions URL for local Ollama.
	pub const COMPLETIONS_URL: &str =
		"http://localhost:11434/v1/chat/completions";

	pub fn o11s(model_slug: impl Into<SmolStr>) -> O11sStreamer {
		O11sStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: model_slug.into(),
			url: Self::RESPONSES_URL.into(),
			auth: None,
		})
	}

	pub fn completions(model_slug: impl Into<SmolStr>) -> CompletionsStreamer {
		CompletionsStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: model_slug.into(),
			url: Self::COMPLETIONS_URL.into(),
			auth: None,
		})
	}


	/// Returns an [`O11sStreamer`] configured for Qwen 3 8B
	/// via the OpenResponses endpoint.
	pub fn qwen() -> O11sStreamer { Self::o11s(Self::QWEN_3_5_9B) }

	/// Returns a [`CompletionsStreamer`] configured for Qwen 3 8B
	/// via the Chat Completions endpoint.
	pub fn qwen_completions() -> CompletionsStreamer {
		Self::completions(Self::QWEN_3_5_9B)
	}
}
