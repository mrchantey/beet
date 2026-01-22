use crate::openresponses;
use beet_core::prelude::*;
use futures::Stream;
pub mod ollama;
pub mod openai;
mod openresponses_provider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use openresponses_provider::*;

/// A trait for providers that implement the OpenResponses API.
///
/// This trait abstracts over different LLM providers (OpenAI, Ollama, etc.)
/// allowing code to work with any compliant backend.
///
/// # Example
///
/// ```no_run
/// use beet_agent::prelude::*;
/// use beet_core::prelude::*;
/// use beet_net::prelude::*;
///
/// # async fn example() -> Result<()> {
/// let mut provider = OllamaProvider::default();
///
/// // Non-streaming request
/// let body = openresponses::RequestBody::new(provider.default_small_model())
///     .with_input("Hello!");
/// let response = provider.send(body).await?;
///
/// // Streaming request
/// let body = openresponses::RequestBody::new(provider.default_small_model())
///     .with_input("Write a poem.")
///     .with_stream(true);
/// let mut stream = provider.stream(body).await?;
///
/// while let Some(event) = stream.next().await {
///     match event? {
///         openresponses::StreamingEvent::OutputTextDelta(ev) => {
///             print!("{}", ev.delta);
///         }
///         openresponses::StreamingEvent::ResponseCompleted(_) => break,
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
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
