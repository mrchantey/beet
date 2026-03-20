use crate::openresponses::request::Input;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

pub struct O11sStreamer {
	action_store: Arc<dyn ActionStore>,
	model: ModelDef,
	/// Whether to use streaming mode.
	stream: bool,
	/// whether to find the previous response if it exists in the thread,
	/// and attempt to pick up where it left off. This should be disabled
	/// for providers who ignore it or are stateless between calls, like ollama
	use_previous_response_id: bool,
	/// System instructions to include with each request.
	instructions: Option<String>,
}

impl std::fmt::Debug for O11sStreamer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("O11sStreamer")
			.field("model", &self.model)
			.field("stream", &self.stream)
			.field("use_previous_response_id", &self.use_previous_response_id)
			.field("instructions", &self.instructions)
			.finish()
	}
}

impl O11sStreamer {
	pub fn new(store: impl 'static + ActionStore, model: ModelDef) -> Self {
		Self {
			action_store: Arc::new(store),
			model,
			stream: true,
			use_previous_response_id: false,
			instructions: None,
		}
	}

	/// Enables streaming mode.
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
		agent_id: ActorId,
		thread_id: ThreadId,
	) -> BoxedFuture<'_, Result<ActionStream>> {
		Box::pin(async move {
			// 1. find last received from this provider match
			// last_received may still be None if no match was found
			let last_received = if self.use_previous_response_id {
				self.action_store
					.previous_meta(
						&self.model.provider_slug,
						&self.model.model_slug,
						thread_id,
					)
					.await?
			} else {
				None
			};

			// 2. build input items
			let items = self
				.action_store
				.full_thread_actions(
					thread_id,
					last_received.as_ref().map(|meta| meta.action_id()),
				)
				.await?
				.into_iter()
				.xtry_map(|(action, author, meta)| {
					o11s_mapper::action_to_o11s_input(
						agent_id, action, author, meta,
					)
				})?;
			// 3. build tool items
			let tools = vec![];

			// 4. build request body
			let mut req_body =
				openresponses::RequestBody::new(&*self.model.url)
					.with_input(Input::Items(items))
					.with_tools(tools)
					.with_stream(self.stream);
			if let Some(last) = last_received {
				req_body = req_body.with_previous_response_id(last.response_id);
			}

			if let Some(instructions) = &self.instructions {
				req_body = req_body.with_instructions(instructions);
			}

			// 5. build request
			let mut request =
				Request::post(&self.model.url)
					.with_json_body::<openresponses::RequestBody>(&req_body)?;
			if let Some(auth) = &self.model.auth {
				request = request.with_auth_bearer(auth);
			}

			let response = request
				.send()
				.await?
				// only accept 2xx and 3xx responses
				.into_result()
				.await?;

			if self.stream {
				let raw_stream = response.event_source_raw().await?;
				Box::pin(O11sStream::new(
					Arc::clone(&self.action_store),
					raw_stream,
				))
			} else {
				let res_body =
					response.json::<openresponses::ResponseBody>().await?;

				let once_stream = futures::stream::once(async {
					Ok(openresponses::StreamingEvent::ResponseCompleted(
						openresponses::streaming::ResponseCompletedEvent {
							sequence_number: 0,
							response: res_body,
						},
					))
				});
				Box::pin(O11sStream::new(
					Arc::clone(&self.action_store),
					once_stream,
				))
			}
			.xok()
		})
	}
}



/// A stream that parses raw SSE events into typed `StreamingEvent` values.
///
/// Handles the `[DONE]` sentinel by cleanly terminating the stream.
struct O11sStream<S> {
	inner: S,
	done: bool,
	action_store: Arc<dyn ActionStore>,
}

impl<S> O11sStream<S> {
	pub(super) fn new(action_store: Arc<dyn ActionStore>, inner: S) -> Self {
		Self {
			action_store,
			inner,
			done: false,
		}
	}
}

impl<S, E> Stream for O11sStream<S>
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
	type Item = Result<ActionStreamOut>;

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
				let o11s_event = serde_json::from_str::<
					openresponses::StreamingEvent,
				>(&event.data)
				.map_err(|err| {
					bevyhow!(
						"Failed to parse streaming event: {}\nRaw: {}",
						err,
						event.data
					)
				})?;

				todo!("handle result");

				Poll::Ready(Some(Ok(vec![])))
			}
			Poll::Ready(Some(Err(err))) => {
				Poll::Ready(Some(Err(bevyhow!("SSE parse error: {}", err))))
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
