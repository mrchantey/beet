//! Ollama provider supporting the OpenResponses API.
//!
//! Ollama provides local LLM inference with OpenResponses-compatible streaming.
use crate::prelude::*;
use beet_core::prelude::*;

/// An OpenResponses-compatible provider for local Ollama inference.
///
/// Ollama must be running locally with OpenResponses API support enabled.
/// By default, connects to `http://localhost:11434/v1/responses`.
#[derive(Debug, Clone, PartialEq, Eq)]
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
	fn box_clone(&self) -> Box<dyn ModelProvider> { Box::new(self.clone()) }

	fn provider_slug(&self) -> &'static str { Self::PROVIDER_SLUG }

	fn default_small_model(&self) -> &'static str { Self::QWEN_3_8B }
	fn default_tool_model(&self) -> &'static str {
		Self::FUNCTION_GEMMA_270M_IT
	}
	fn default_large_model(&self) -> &'static str {
		Self::QWEN_3_ABLITERATED_14B
	}

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


impl OllamaProvider {
	pub const PROVIDER_SLUG: &str = "ollama";
	/// Qwen 3 Abliterated 14B - large uncensored model.
	pub const QWEN_3_ABLITERATED_14B: &str = "huihui_ai/qwen3-abliterated:14b";
	/// Function Gemma 270M IT - small function calling model.
	pub const FUNCTION_GEMMA_270M_IT: &str = "functiongemma:270m-it-fp16";
	/// Qwen 3 8B - balanced model.
	pub const QWEN_3_8B: &str = "qwen3:8b";

	/// Default responses URL for local Ollama.
	pub const RESPONSES_URL: &str = "http://localhost:11434/v1/responses";


	pub fn qwen_3_8b() -> O11sStreamer {
		O11sStreamer::new(ModelDef {
			provider_slug: Self::PROVIDER_SLUG.into(),
			model_slug: Self::QWEN_3_8B.into(),
			url: Self::RESPONSES_URL.into(),
			auth: None,
		})
	}
}
