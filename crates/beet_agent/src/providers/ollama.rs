//! Ollama provider supporting the OpenResponses API.
//!
//! Ollama provides local LLM inference with OpenResponses-compatible streaming.
//!
//! # Example
//!
//! ```no_run
//! use beet_agent::prelude::*;
//! use beet_core::prelude::*;
//! use beet_net::prelude::*;
//!
//! # async fn example() -> Result<()> {
//! let mut provider = OllamaProvider::default();
//!
//! // Non-streaming request
//! let body = openresponses::RequestBody::new(provider.default_small_model())
//!     .with_input("Hello!");
//! let response = provider.send(body).await?;
//!
//! // Streaming request
//! let body = openresponses::RequestBody::new(provider.default_small_model())
//!     .with_input("Write a poem.")
//!     .with_stream(true);
//! let mut stream = provider.stream(body).await?;
//!
//! while let Some(event) = stream.next().await {
//!     match event? {
//!         openresponses::StreamingEvent::OutputTextDelta(ev) => {
//!             print!("{}", ev.delta);
//!         }
//!         openresponses::StreamingEvent::ResponseCompleted(_) => break,
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use futures::Stream;
use std::borrow::Cow;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;


pub mod models {
	pub const QWEN_3_ABLITERATED_14B: &str = "huihui_ai/qwen3-abliterated:14b";
	pub const FUNCTION_GEMMA_270M_IT: &str = "functiongemma:270m-it-fp16";
	pub const QWEN_3_8B: &str = "qwen3:8b";
}

/// An OpenResponses-compatible provider for local Ollama inference.
///
/// Ollama must be running locally with OpenResponses API support enabled.
/// By default, connects to `http://localhost:11434/v1/responses`.
pub struct OllamaProvider {
	/// The full URL to the OpenResponses-compatible Ollama endpoint.
	/// Defaults to `http://localhost:11434/v1/responses`.
	url: Cow<'static, str>,
}

impl Default for OllamaProvider {
	fn default() -> Self {
		Self {
			url: "http://localhost:11434/v1/responses".into(),
		}
	}
}

impl OllamaProvider {
	/// Creates a new provider with a custom endpoint URL.
	pub fn with_url(url: impl Into<Cow<'static, str>>) -> Self {
		Self { url: url.into() }
	}
}

impl ModelProvider for OllamaProvider {
	fn default_small_model(&self) -> &'static str { models::QWEN_3_8B }
	fn default_tool_model(&self) -> &'static str {
		models::FUNCTION_GEMMA_270M_IT
	}
	fn default_large_model(&self) -> &'static str {
		models::QWEN_3_ABLITERATED_14B
	}

	fn send(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<openresponses::ResponseBody>> {
		async move {
			if request.stream == Some(true) {
				bevybail!(
					"streaming must not be enabled in the request to use the send method"
				);
			}
			Request::post(&self.url)
				.with_json_body::<openresponses::RequestBody>(&request)
				.unwrap()
				.send()
				.await?
				.into_result()
				.await?
				.json::<openresponses::ResponseBody>()
				.await
		}
	}

	fn stream(
		&mut self,
		request: openresponses::RequestBody,
	) -> impl Future<
		Output = Result<
			impl Stream<Item = Result<openresponses::StreamingEvent>>,
		>,
	> {
		async move {
			if request.stream != Some(true) {
				bevybail!(
					"streaming must be enabled in the request to use the stream method"
				);
			}
			let raw_stream = Request::post(&self.url)
				.with_json_body::<openresponses::RequestBody>(&request)
				.unwrap()
				.send()
				.await?
				.event_source_raw()
				.await?;

			OpenResponsesStream::new(raw_stream).xok()
		}
	}
}

/// A stream that parses raw SSE events into typed `StreamingEvent` values.
///
/// Handles the `[DONE]` sentinel by cleanly terminating the stream.
struct OpenResponsesStream<S> {
	inner: S,
	done: bool,
}

impl<S> OpenResponsesStream<S> {
	fn new(inner: S) -> Self { Self { inner, done: false } }
}

impl<S, E> Stream for OpenResponsesStream<S>
where
	S: Stream<
			Item = std::result::Result<
				beet_net::exports::eventsource_stream::Event,
				E,
			>,
		> + Unpin,
	E: std::fmt::Display,
{
	type Item = Result<openresponses::StreamingEvent>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		match Pin::new(&mut self.inner).poll_next(cx) {
			Poll::Ready(Some(Ok(event))) => {
				// Handle the [DONE] sentinel
				if event.data == "[DONE]" {
					self.done = true;
					return Poll::Ready(None);
				}

				// Parse the event data as a StreamingEvent
				let result = serde_json::from_str::<
					openresponses::StreamingEvent,
				>(&event.data)
				.map_err(|err| {
					bevyhow!(
						"Failed to parse streaming event: {}\nRaw: {}",
						err,
						event.data
					)
				});
				Poll::Ready(Some(result))
			}
			Poll::Ready(Some(Err(err))) => {
				Poll::Ready(Some(Err(bevyhow!("SSE parse error: {}", err))))
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
