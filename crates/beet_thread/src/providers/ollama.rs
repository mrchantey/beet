//! Ollama provider supporting the OpenResponses API.
//!
//! Ollama provides local LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;

/// An OpenResponses-compatible provider for local Ollama inference.
///
/// Ollama must be running locally with OpenResponses API support enabled.
/// By default, connects to `http://localhost:11434/v1/responses`.
pub struct OllamaProvider;

impl OllamaProvider {
	pub const PROVIDER_SLUG: &str = "ollama";
	/// Qwen 3 Abliterated 14B - large uncensored model.
	pub const QWEN_3_ABLITERATED_14B: &str = "huihui_ai/qwen3-abliterated:14b";
	/// Function Gemma 270M IT - small function calling model.
	pub const FUNCTION_GEMMA_270M_IT: &str = "functiongemma:270m-it-fp16";
	/// Qwen 3 8B - balanced model.
	pub const QWEN_3_8B: &str = "qwen3:8b";

	/// Default OpenResponses URL for local Ollama.
	pub const RESPONSES_URL: &str = "http://localhost:11434/v1/responses";
	/// Default Chat Completions URL for local Ollama.
	pub const COMPLETIONS_URL: &str =
		"http://localhost:11434/v1/chat/completions";

	/// Returns an [`O11sStreamer`] configured for Qwen 3 8B
	/// via the OpenResponses endpoint.
	pub fn qwen_3_8b() -> O11sStreamer {
		O11sStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::QWEN_3_8B.into(),
			url: Self::RESPONSES_URL.into(),
			auth: None,
		})
	}

	/// Returns a [`CompletionsStreamer`] configured for Qwen 3 8B
	/// via the Chat Completions endpoint.
	pub fn qwen_3_8b_completions() -> CompletionsStreamer {
		CompletionsStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::QWEN_3_8B.into(),
			url: Self::COMPLETIONS_URL.into(),
			auth: None,
		})
	}
}
