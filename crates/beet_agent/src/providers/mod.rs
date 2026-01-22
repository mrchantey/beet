use crate::openresponses;
use beet_core::prelude::*;
use futures::Stream;
pub mod ollama;
pub use ollama::OllamaProvider;

/// A trait for providers that implement the OpenResponses API.
///
/// This trait abstracts over different LLM providers (OpenAI, Ollama, etc.)
/// allowing code to work with any compliant backend.
pub trait ModelProvider {
	/// The recommended model from this provider for short and simple
	/// requests and responses.
	fn default_small_model(&self) -> &'static str;
	/// The recommended model from this provider for simple tool calls.
	fn default_tool_model(&self) -> &'static str;
	/// The recommended model from this provider for complex tasks and reasoning.
	fn default_large_model(&self) -> &'static str;

	/// Sends a non-streaming request and returns the complete response.
	fn send(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<openresponses::ResponseBody>>;

	/// Sends a streaming request and returns a stream of typed events.
	///
	/// The request must have `stream: true` set, otherwise this will error.
	/// The returned stream yields strongly-typed [`StreamingEvent`](openresponses::StreamingEvent)
	/// values and terminates cleanly when the `[DONE]` sentinel is received.
	fn stream(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<
		Output = Result<
			impl Stream<Item = Result<openresponses::StreamingEvent>>,
		>,
	>;
}
