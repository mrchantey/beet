//! Ollama provider supporting the OpenResponses API.
//!
//! Ollama provides local LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;


pub mod constants {
	pub const QWEN_3_ABLITERATED_14B: &str = "huihui_ai/qwen3-abliterated:14b";
	pub const FUNCTION_GEMMA_270M_IT: &str = "functiongemma:270m-it-fp16";
	pub const QWEN_3_8B: &str = "qwen3:8b";

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
			inner: OpenResponsesProvider::new(constants::RESPONSES_URL),
		}
	}
}

impl OllamaProvider {}

impl ModelProvider for OllamaProvider {
	fn provider_slug(&self) -> &'static str { "ollama" }

	fn default_small_model(&self) -> &'static str { constants::QWEN_3_8B }
	fn default_tool_model(&self) -> &'static str {
		constants::FUNCTION_GEMMA_270M_IT
	}
	fn default_large_model(&self) -> &'static str {
		constants::QWEN_3_ABLITERATED_14B
	}

	fn send(
		&self,
		request: openresponses::RequestBody,
	) -> BoxFuture<'_, Result<openresponses::ResponseBody>> {
		Box::pin(self.inner.send(request))
	}

	fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> BoxFuture<'_, Result<StreamingEventStream>> {
		Box::pin(self.inner.stream(request))
	}
}
