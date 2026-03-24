use crate::o11s;
use beet_core::prelude::*;
use futures::Stream;
use std::pin::Pin;

/// A boxed, pinned stream of streaming events.
///
/// This type alias provides ergonomic stream handling without requiring
/// callers to manually pin the stream.
pub type StreamingEventStream =
	Pin<Box<dyn Stream<Item = Result<o11s::StreamingEvent>> + Send>>;

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
/// use beet_actor::prelude::*;
/// use beet_core::prelude::*;
/// use beet_net::prelude::*;
///
/// # async fn example() -> Result<()> {
/// let mut provider = OllamaProvider::default();
///
/// // Non-streaming request
/// let body = o11s::RequestBody::new(provider.default_small_model())
///     .with_simple_input("Hello!");
/// let response = provider.send(body).await?;
///
/// // Streaming request
/// let body = o11s::RequestBody::new(provider.default_small_model())
///     .with_simple_input("Write a poem.")
///     .with_stream(true);
/// let mut stream = provider.stream(body).await?;
///
/// while let Some(event) = stream.next().await {
///     match event? {
///         o11s::StreamingEvent::OutputTextDelta(ev) => {
///             print!("{}", ev.delta);
///         }
///         o11s::StreamingEvent::ResponseCompleted(_) => break,
///         _ => {}
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub trait ModelProvider: 'static + Send + Sync {
	fn box_clone(&self) -> Box<dyn ModelProvider>;

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
		request: o11s::RequestBody,
	) -> BoxedFuture<'_, Result<o11s::ResponseBody>>;

	/// Sends a streaming request and returns a pinned stream of typed events.
	///
	/// The request must have `stream: true` set, otherwise this will error.
	/// The returned stream yields strongly-typed [`StreamingEvent`](o11s::StreamingEvent)
	/// values and terminates cleanly when the `[DONE]` sentinel is received.
	///
	/// The stream is returned pre-pinned for ergonomic use - no `pin!()` macro needed.
	fn stream(
		&self,
		request: o11s::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>>;
}

/// Alias for a boxed, sendable model provider.
pub type BoxedModelProvider = Box<dyn ModelProvider>;

impl ModelProvider for Box<dyn ModelProvider> {
	fn box_clone(&self) -> Box<dyn ModelProvider> { self.as_ref().box_clone() }

	fn provider_slug(&self) -> &'static str { self.as_ref().provider_slug() }

	fn default_small_model(&self) -> &'static str {
		self.as_ref().default_small_model()
	}

	fn default_tool_model(&self) -> &'static str {
		self.as_ref().default_tool_model()
	}

	fn default_large_model(&self) -> &'static str {
		self.as_ref().default_large_model()
	}

	fn send(
		&self,
		request: o11s::RequestBody,
	) -> BoxedFuture<'_, Result<o11s::ResponseBody>> {
		self.as_ref().send(request)
	}

	fn stream(
		&self,
		request: o11s::RequestBody,
	) -> BoxedFuture<'_, Result<StreamingEventStream>> {
		self.as_ref().stream(request)
	}
}
