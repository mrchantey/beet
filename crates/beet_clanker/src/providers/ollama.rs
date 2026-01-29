//! Ollama provider supporting the OpenResponses API.
//!
//! Ollama provides local LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;

impl OllamaProvider {
	/// Qwen 3 Abliterated 14B - large uncensored model.
	pub const QWEN_3_ABLITERATED_14B: &str = "huihui_ai/qwen3-abliterated:14b";
	/// Function Gemma 270M IT - small function calling model.
	pub const FUNCTION_GEMMA_270M_IT: &str = "functiongemma:270m-it-fp16";
	/// Qwen 3 8B - balanced model.
	pub const QWEN_3_8B: &str = "qwen3:8b";

	/// Default responses URL for local Ollama.
	pub const RESPONSES_URL: &str = "http://localhost:11434/v1/responses";
}

/// An OpenResponses-compatible provider for local Ollama inference.
///
/// Ollama must be running locally with OpenResponses API support enabled.
/// By default, connects to `http://localhost:11434/v1/responses`.
pub struct OllamaProvider {
	/// The full URL to the OpenResponses-compatible Ollama endpoint.
	/// Defaults to `http://localhost:11434/v1/responses`.
	inner: OpenResponsesProvider,
}

impl Default for OllamaProvider {
	fn default() -> Self {
		Self {
			inner: OpenResponsesProvider::new(Self::RESPONSES_URL),
		}
	}
}

impl OllamaProvider {}

impl ModelProvider for OllamaProvider {
	fn provider_slug(&self) -> &'static str { "ollama" }

	fn default_small_model(&self) -> &'static str { Self::QWEN_3_8B }
	fn default_tool_model(&self) -> &'static str {
		Self::FUNCTION_GEMMA_270M_IT
	}
	fn default_large_model(&self) -> &'static str {
		Self::QWEN_3_ABLITERATED_14B
	}

	fn send(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<openresponses::ResponseBody>> {
		let request = OpenResponsesProvider::inline_text_file_data(request);
		Box::pin(self.inner.send(request))
	}

	fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>> {
		let request = OpenResponsesProvider::inline_text_file_data(request);
		Box::pin(self.inner.stream(request))
	}
}
