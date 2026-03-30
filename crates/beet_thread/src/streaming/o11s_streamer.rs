use crate::o11s::request::Input;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_tool::prelude::*;
use futures::Stream;
use std::borrow::Cow;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

#[derive(Debug, Clone, Component)]
#[component(on_add=on_add)]
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

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(async_tool(post_streamer_tool::<O11sStreamer>));
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

	/// Sets the instructions for this streamer.
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

	async fn build_request(
		&self,
		caller: AsyncEntity,
	) -> Result<(o11s::RequestBody, ActorId, ThreadId)> {
		// TODO perhaps with new bevy async we wont need 'static FnOnce,
		// so wont need to clone
		let this = self.clone();

		caller
			.with_state::<SocialQuery, _>(
				move |actor_entity,
				      query|
				      -> Result<(o11s::RequestBody, ActorId, ThreadId)> {
					let thread = query.thread(actor_entity)?;
					let agent = thread.actor(actor_entity)?;

					// get last received response meta
					let last_received = if this.use_previous_response_id {
						thread.stored_response(
							actor_entity,
							&this.model.provider_slug,
							&this.model.model_slug,
						)
					} else {
						None
					};

					// get input items (from last received if caching)
					let items = thread
						.posts_from(last_received.map(|(post, _)| post.id()))
						.into_iter()
						.xtry_map(|post| {
							o11s_mapper::post_to_o11s_input(agent.id(), post)
						})?;

					let tools = query
						.tools(agent.entity)
						.into_iter()
						.map(|(_entity, tool_def)| {
							o11s_mapper::tool_to_function_param(tool_def)
						})
						.collect::<Vec<_>>();

					// last_received.map(|(_, meta)| meta.clone())
					// 4. build request body
					let mut req_body =
						o11s::RequestBody::new(&*this.model.model_slug)
							.with_input(Input::Items(items))
							.with_tools(tools)
							.with_stream(this.stream);

					if let Some(tool_choice) = &agent.tool_choice {
						req_body = req_body.with_tool_choice(
							o11s_mapper::tool_choice(tool_choice),
						);
					}

					if let Some((_, last)) = last_received {
						req_body = req_body
							.with_previous_response_id(&last.response_id);
					}
					if let Some(instructions) = this.instructions {
						req_body = req_body.with_instructions(instructions);
					}
					(req_body, agent.id(), thread.id()).xok()
				},
			)
			.await
	}
}

impl PostStreamer for O11sStreamer {
	fn provider_slug(&self) -> Cow<'static, str> {
		self.model.provider_slug.clone()
	}
	fn model_slug(&self) -> Cow<'static, str> { self.model.model_slug.clone() }


	fn stream_posts(
		&mut self,
		caller: AsyncEntity,
	) -> BoxedFuture<'_, Result<PostStream>> {
		Box::pin(async move {
			let (req_body, agent, thread) = self.build_request(caller).await?;

			// 5. build and send request
			let mut request = Request::post(&self.model.url)
				.with_json_body::<o11s::RequestBody>(&req_body)?;
			if let Some(auth) = &self.model.auth {
				request = request.with_auth_bearer(auth);
			}
			let response = request.send().await?.into_result().await?;

			// 6. unify streaming and non-streaming into a single typed stream
			let typed_stream: ResPartialStream = if self.stream {
				let raw_stream = response.event_source_raw().await?;
				Box::pin(O11sSseStream::new(raw_stream))
			} else {
				let res_body = response.json::<o11s::ResponseBody>().await?;
				trace!("Received full response body: {:#?}", res_body);
				// coherse a oneshot into a 'completed' sse event
				let res_partial = o11s_mapper::response_to_partial(res_body)?;
				Box::pin(futures::stream::once(async move { Ok(res_partial) }))
			};
			PostStream::new(
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
struct O11sSseStream<S> {
	inner: S,
	done: bool,
	prev_state: Option<ResponsePartial>,
}

impl<S> O11sSseStream<S> {
	fn new(inner: S) -> Self {
		Self {
			inner,
			done: false,
			prev_state: None,
		}
	}
}

impl<S, E> Stream for O11sSseStream<S>
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
				trace!("Received raw SSE event: {:#?}", event);
				if event.data.trim() == "[DONE]" {
					// do not attempt reconnect
					self.done = true;
					return Poll::Ready(None);
				}
				match event.event.as_str().trim() {
					"response.completed"
					| "response.failed"
					| "response.incomplete" => {
						self.done = true;
					}
					_ev => {}
				}
				let ev_result =
					serde_json::from_str::<o11s::StreamingEvent>(&event.data)
						.map_err(|err| {
							bevyhow!(
								"Failed to parse streaming event: {}\nRaw: {}",
								err,
								event.data
							)
						});
				// if matches!(ev_result,Ok(o11s::StreamingEvent::ResponseCreated(_)))

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
