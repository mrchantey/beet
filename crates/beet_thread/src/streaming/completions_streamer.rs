//! A [`PostStreamer`] implementation targeting the OpenAI Chat Completions API.
//!
//! Used by providers that support the completions endpoint but not
//! the OpenResponses protocol, ie Gemini.
use crate::prelude::*;
use crate::streaming::completions_mapper;
use async_openai::types::chat::CreateChatCompletionRequest;
use async_openai::types::chat::CreateChatCompletionResponse;
use async_openai::types::chat::CreateChatCompletionStreamResponse;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use futures::Stream;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

/// Streams responses from an OpenAI-compatible Chat Completions endpoint,
/// mapping them into [`PostStream`] values via
/// [`completions_mapper`](super::completions_mapper).
#[derive(Debug, Clone, Component, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize, Component)]
#[require(Action<(),Outcome> = Self::default_action())]
pub struct CompletionsStreamer {
	model: ModelDef,
	/// Whether to use streaming mode.
	stream: bool,
	/// System instructions to include with each request.
	instructions: Option<String>,
}

impl DefaultAction<(), Outcome> for CompletionsStreamer {
	fn default_action() -> Action<(), Outcome> {
		Action::new_async(post_streamer_action::<CompletionsStreamer>)
	}
}

impl IntoAction<Self> for CompletionsStreamer {
	type In = ();
	type Out = Outcome;
	fn into_action(self) -> Action<(), Outcome> {
		Action::new_async(async move |cx: ActionContext| {
			post_streamer_action_stateful(cx.map_input(self)).await
		})
	}
}

impl CompletionsStreamer {
	pub fn new(model: ModelDef) -> Self {
		Self {
			model,
			stream: true,
			instructions: None,
		}
	}

	/// Disables streaming mode, returning the full response as a single event.
	pub fn without_streaming(mut self) -> Self {
		self.stream = false;
		self
	}

	/// Sets system instructions for this streamer.
	pub fn with_instructions(
		mut self,
		instructions: impl Into<String>,
	) -> Self {
		self.instructions = Some(instructions.into());
		self
	}

	/// Builds a [`CreateChatCompletionRequest`] from the current ECS state.
	async fn build_request(
		&self,
		caller: AsyncEntity,
	) -> Result<(CreateChatCompletionRequest, ActorId, ThreadId)> {
		let this = self.clone();

		caller
			.with_state::<ThreadQuery, _>(
				move |actor_entity,
				      query|
				      -> Result<(
					CreateChatCompletionRequest,
					ActorId,
					ThreadId,
				)> {
					let thread = query.thread(actor_entity)?;
					let agent = thread.actor(actor_entity)?;

					let mut messages = Vec::new();

					// Inject system instructions as the first message
					if let Some(instructions) = &this.instructions {
						use async_openai::types::chat::*;
						messages.push(
							ChatCompletionRequestMessage::System(
								ChatCompletionRequestSystemMessage {
									content:
										ChatCompletionRequestSystemMessageContent::Text(
											instructions.clone(),
										),
									name: None,
								},
							),
						);
					}

					// Convert thread posts to completions messages
					let post_messages =
						thread.posts.iter().xtry_map(|post| {
							completions_mapper::post_to_completions_message(
								agent.id(),
								post.clone(),
							)
						})?;
					messages.extend(post_messages);

					// Collect tools
					let tools = query
						.tools(agent.entity)
						.into_iter()
						.map(|(_entity, tool_def)| {
							completions_mapper::tool_to_completions_tool(
								tool_def,
							)
						})
						.collect::<Vec<_>>();

					let tool_choice = agent.tool_choice.map(|choice| {
						completions_mapper::tool_choice_to_completions(choice)
					});

					#[allow(deprecated)]
					let req = CreateChatCompletionRequest {
						model: this.model.model_slug.to_string(),
						messages,
						stream: Some(this.stream),
						tools: if tools.is_empty() {
							None
						} else {
							Some(tools)
						},
						tool_choice,
						stream_options: if this.stream {
							Some(
									async_openai::types::chat::ChatCompletionStreamOptions {
										include_usage: Some(true),
										include_obfuscation: None,
									},
								)
						} else {
							None
						},
						// Defaults for the rest
						modalities: None,
						verbosity: None,
						reasoning_effort: None,
						max_completion_tokens: None,
						frequency_penalty: None,
						presence_penalty: None,
						web_search_options: None,
						top_logprobs: None,
						response_format: None,
						audio: None,
						store: None,
						stop: None,
						logit_bias: None,
						logprobs: None,
						max_tokens: None,
						n: None,
						prediction: None,
						seed: None,
						service_tier: None,
						temperature: None,
						top_p: None,
						parallel_tool_calls: None,
						user: None,
						safety_identifier: None,
						prompt_cache_key: None,
						function_call: None,
						functions: None,
						metadata: None,
					};

					(req, agent.id(), thread.id()).xok()
				},
			)
			.await
	}
}

impl PostStreamer for CompletionsStreamer {
	fn provider_slug(&self) -> &str { &self.model.provider_slug }
	fn model_slug(&self) -> &str { &self.model.model_slug }

	fn stream_posts(
		&self,
		caller: AsyncEntity,
	) -> BoxedFuture<'_, Result<PostStream>> {
		Box::pin(async move {
			let (req_body, agent, thread) = self.build_request(caller).await?;

			let mut request = Request::post(self.model.url.as_str())
				.with_json_body(&req_body)?;
			if let Some(auth) = &self.model.auth {
				request = request.with_auth_bearer(auth.value());
			}
			let response = request.send().await?.into_result().await?;

			let typed_stream: ResPartialStream = if self.stream {
				let raw_stream = response.event_source_raw().await?;
				Box::pin(CompletionsSseStream::new(raw_stream))
			} else {
				let res: CreateChatCompletionResponse =
					response.json::<CreateChatCompletionResponse>().await?;
				trace!("Received full completions response: {:#?}", res);
				let partial = completions_mapper::response_to_partial(res)?;
				Box::pin(futures::stream::once(async move { Ok(partial) }))
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


// ═══════════════════════════════════════════════════════════════════════
// SSE -> ResponsePartial stream adapter
// ═══════════════════════════════════════════════════════════════════════

/// Parses raw SSE events from a completions streaming endpoint into
/// [`ResponsePartial`] values.
struct CompletionsSseStream<S> {
	inner: S,
	done: bool,
	accumulator: StreamAccumulator,
}

impl<S> CompletionsSseStream<S> {
	fn new(inner: S) -> Self {
		Self {
			inner,
			done: false,
			accumulator: StreamAccumulator::new(),
		}
	}
}

impl<S, E> Stream for CompletionsSseStream<S>
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
				trace!("Received completions SSE event: {:#?}", event);
				if event.data.trim() == "[DONE]" {
					self.done = true;
					return Poll::Ready(None);
				}

				let chunk = match serde_json::from_str::<
					CreateChatCompletionStreamResponse,
				>(&event.data)
				{
					Ok(chunk) => chunk,
					Err(err) => {
						return Poll::Ready(Some(Err(bevyhow!(
							"Failed to parse completions stream chunk: {}\nRaw: {}",
							err,
							event.data
						))));
					}
				};

				let partial = completions_mapper::stream_chunk_to_partial(
					chunk,
					&mut self.accumulator,
				);

				// Mark done if the response is final
				if let Ok(ref res) = partial {
					if res.is_final() {
						self.done = true;
					}
				}

				Poll::Ready(Some(partial))
			}
			Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(bevyhow!(
				"Completions SSE error: {}",
				err
			)))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}



/// Tracks accumulated tool call state across streaming chunks.
/// Each tool call arrives incrementally: the first chunk carries `id`
/// and `name`, subsequent chunks append to `arguments`.
#[derive(Debug, Clone, Default)]
pub struct StreamAccumulator {
	/// In-flight tool calls keyed by chunk index.
	pub tool_calls: Vec<AccumulatedToolCall>,
}

/// A single in-progress tool call being assembled from stream deltas.
#[derive(Debug, Clone, Default)]
pub struct AccumulatedToolCall {
	/// The tool call index within the response.
	pub index: u32,
	/// Tool call ID, provided in the first chunk.
	pub id: String,
	/// Function name, provided in the first chunk.
	pub name: String,
	/// Arguments accumulated across deltas.
	pub arguments: String,
}

impl StreamAccumulator {
	pub fn new() -> Self { Self::default() }

	/// Resets state for a new response stream.
	pub fn reset(&mut self) { self.tool_calls.clear(); }

	pub fn get_or_insert(&mut self, index: u32) -> &mut AccumulatedToolCall {
		if let Some(pos) =
			self.tool_calls.iter().position(|tc| tc.index == index)
		{
			&mut self.tool_calls[pos]
		} else {
			self.tool_calls.push(AccumulatedToolCall {
				index,
				..Default::default()
			});
			self.tool_calls.last_mut().unwrap()
		}
	}
}
