use crate::openresponses::request::Input;
use crate::prelude::*;
use crate::types::ResPartialStream;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

#[derive(Debug, Clone, Component)]
pub struct O11sStreamer {
	model: ModelDef,
	/// Whether to use streaming mode.
	stream: bool,
	/// Whether to find the previous response if it exists in the thread,
	/// and attempt to pick up where it left off. This should be disabled
	/// for providers who ignore it or are stateless between calls, like ollama.
	use_previous_response_id: bool,
	/// System instructions to include with each request.
	instructions: Option<String>,
}

impl O11sStreamer {
	pub fn new(model: ModelDef) -> Self {
		Self {
			model,
			stream: true,
			use_previous_response_id: false,
			instructions: None,
		}
	}

	/// Disables streaming mode, returning the full response as a single event.
	pub fn without_streaming(mut self) -> Self {
		self.stream = false;
		self
	}

	/// Sets the instructions for this model action.
	pub fn with_instructions(
		mut self,
		instructions: impl Into<String>,
	) -> Self {
		self.instructions = Some(instructions.into());
		self
	}

	pub fn with_use_previous_response_id(mut self) -> Self {
		self.use_previous_response_id = true;
		self
	}
}

impl ActionStreamer for O11sStreamer {
	fn stream_actions(
		&mut self,
		action_store: impl ActionStoreProvider,
		agent: ActorId,
		thread: ThreadId,
	) -> BoxedFuture<'_, Result<ActionStream>> {
		Box::pin(async move {
			// 1. find last received from this provider match
			// last_received may still be None if no match was found
			let last_received = if self.use_previous_response_id {
				action_store
					.stored_response_meta(
						&self.model.provider_slug,
						&self.model.model_slug,
						thread,
					)
					.await?
			} else {
				None
			};

			// 2. build input items
			let input_items = action_store
				.full_thread_actions(
					thread,
					last_received.as_ref().map(|meta| meta.action_id()),
				)
				.await?
				.into_iter()
				.xtry_map(|(action, author)| {
					o11s_mapper::action_to_o11s_input(agent, action, author)
				})?;

			// 3. build tool items
			let tools = vec![];

			// 4. build request body
			let mut req_body =
				openresponses::RequestBody::new(&*self.model.model_slug)
					.with_input(Input::Items(input_items))
					.with_tools(tools)
					.with_stream(self.stream);
			if let Some(last) = last_received {
				req_body = req_body.with_previous_response_id(last.response_id);
			}
			if let Some(instructions) = &self.instructions {
				req_body = req_body.with_instructions(instructions);
			}

			// 5. build and send request
			let mut request =
				Request::post(&self.model.url)
					.with_json_body::<openresponses::RequestBody>(&req_body)?;
			if let Some(auth) = &self.model.auth {
				request = request.with_auth_bearer(auth);
			}
			let response = request.send().await?.into_result().await?;

			// 6. unify streaming and non-streaming into a single typed stream
			let typed_stream: ResPartialStream = if self.stream {
				let raw_stream = response.event_source_raw().await?;
				Box::pin(SseToTypedStream::new(raw_stream))
			} else {
				let res_body =
					response.json::<openresponses::ResponseBody>().await?;
				// coherse a oneshot into a 'completed' sse event
				let res_partial = o11s_mapper::response_to_partial(res_body)?;
				Box::pin(futures::stream::once(async move { Ok(res_partial) }))
			};
			ActionStream::new(
				self.model.provider_slug.clone(),
				self.model.model_slug.clone(),
				agent,
				thread,
				typed_stream,
			)
			.xok()
		})
	}
}

/// Parses raw SSE events into typed [`StreamingEvent`](openresponses::StreamingEvent) values.
///
/// Handles the `[DONE]` sentinel by cleanly terminating the stream.
struct SseToTypedStream<S> {
	inner: S,
	done: bool,
	prev_state: Option<ResponsePartial>,
}

impl<S> SseToTypedStream<S> {
	fn new(inner: S) -> Self {
		Self {
			inner,
			done: false,
			prev_state: None,
		}
	}
}

impl<S, E> Stream for SseToTypedStream<S>
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
	type Item = Result<ResponsePartial>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}
		match Pin::new(&mut self.inner).poll_next(cx) {
			Poll::Ready(Some(Ok(event))) => {
				if event.data == "[DONE]" {
					self.done = true;
					return Poll::Ready(None);
				}
				let ev_result = serde_json::from_str::<
					openresponses::StreamingEvent,
				>(&event.data)
				.map_err(|err| {
					bevyhow!(
						"Failed to parse streaming event: {}\nRaw: {}",
						err,
						event.data
					)
				});

				let res_partial = ev_result
					.map(|ev| {
						o11s_mapper::ev_to_response_partial(
							self.prev_state.take(),
							ev,
						)
					})
					.flatten();

				if let Ok(ref partial) = res_partial {
					self.prev_state = Some(partial.clone());
				}

				Poll::Ready(Some(res_partial))
			}
			Poll::Ready(Some(Err(err))) => {
				Poll::Ready(Some(Err(bevyhow!("SSE parse error: {}", err))))
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
