use crate::openresponses;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::pin::Pin;

/// A boxed, pinned stream of streaming events.
///
/// This type alias provides ergonomic stream handling without requiring
/// callers to manually pin the stream.
pub type StreamingEventStream =
	Pin<Box<dyn Stream<Item = Result<openresponses::StreamingEvent>> + Send>>;

/// A trait for providers that implement the OpenResponses API.
///
/// This trait abstracts over different LLM providers (OpenAI, Ollama, etc.)
/// allowing code to work with any compliant backend.
///
/// The trait is object-safe and can be used with `Box<dyn ModelProvider>`.
///
/// # Example
///
/// ```no_run
/// use beet_clanker::prelude::*;
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
pub trait ModelProvider: 'static + Send + Sync {
	/// A short slug identifying this provider (e.g., "openai", "ollama").
	fn provider_slug(&self) -> &'static str;

	/// The recommended model from this provider for short and simple
	/// requests and responses.
	fn default_small_model(&self) -> &'static str;
	/// The recommended model from this provider for simple tool calls.
	fn default_tool_model(&self) -> &'static str;
	/// The recommended model from this provider for complex tasks and reasoning.
	fn default_large_model(&self) -> &'static str;

	/// Sends a non-streaming request and returns the complete response.
	fn send(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<openresponses::ResponseBody>>;

	/// Sends a streaming request and returns a pinned stream of typed events.
	///
	/// The request must have `stream: true` set, otherwise this will error.
	/// The returned stream yields strongly-typed [`StreamingEvent`](openresponses::StreamingEvent)
	/// values and terminates cleanly when the `[DONE]` sentinel is received.
	///
	/// The stream is returned pre-pinned for ergonomic use - no `pin!()` macro needed.
	fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>>;
}

/// Alias for a boxed, sendable model provider.
pub type BoxedModelProvider = Box<dyn ModelProvider>;
