//! Shared utilities for openresponses-compliant providers
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use futures::Stream;
use std::borrow::Cow;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

pub struct OpenResponsesProvider {
	auth: Option<Cow<'static, str>>,
	url: Cow<'static, str>,
}

impl OpenResponsesProvider {
	pub fn new(url: impl Into<Cow<'static, str>>) -> Self {
		Self {
			url: url.into(),
			auth: None,
		}
	}
	pub fn with_auth(mut self, auth: impl Into<Cow<'static, str>>) -> Self {
		self.auth = Some(auth.into());
		self
	}

	fn build_request(
		&self,
		request: openresponses::RequestBody,
	) -> Result<Request> {
		let mut request = Request::post(&self.url)
			.with_json_body::<openresponses::RequestBody>(&request)?;
		if let Some(auth) = &self.auth {
			request = request.with_auth_bearer(auth);
		}
		request.xok()
	}

	pub fn send(
		&self,
		request: openresponses::RequestBody,
	) -> impl Future<Output = Result<openresponses::ResponseBody>> {
		async move {
			if request.stream == Some(true) {
				bevybail!(
					"streaming must not be enabled in the request to use the send method"
				);
			}
			self.build_request(request)?
				.send()
				.await?
				.into_result()
				.await?
				.json::<openresponses::ResponseBody>()
				.await
		}
	}

	pub async fn stream(
		&self,
		request: openresponses::RequestBody,
	) -> Result<StreamingEventStream> {
		if request.stream != Some(true) {
			bevybail!(
				"streaming must be enabled in the request to use the stream method"
			);
		}
		let raw_stream = self
			.build_request(request)?
			.send()
			.await?
			.event_source_raw()
			.await?;

		let stream: StreamingEventStream =
			Box::pin(OpenResponsesStream::new(raw_stream));
		stream.xok()
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
	pub(super) fn new(inner: S) -> Self { Self { inner, done: false } }
}

impl<S, E> Stream for OpenResponsesStream<S>
where
	S: Stream<
			Item = std::result::Result<
				beet_net::exports::eventsource_stream::Event,
				E,
			>,
		> + Unpin
		+ Send,
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
