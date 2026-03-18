//! Model action component for AI agent interactions.
//!
//! The [`ModelAction`] component configures how an action entity interacts
//! with an AI model. When added to an entity, it automatically inserts the
//! request behavior via an `on_add` hook.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;
use std::ops::ControlFlow;

/// A component that configures an action entity to interact with an AI model.
///
/// When this component is added to an entity, it automatically inserts
/// the model action request behavior via an `on_add` hook.
///
/// # Multi-Agent Conversations
///
/// Each `ModelAction` maintains its own `previous_response_id` for multi-turn
/// conversations. This allows multiple agents in the same behavior tree to
/// have independent conversation state.
///
/// When `previous_response_id` is set, context entities that have already been
/// sent (tracked in `sent_context_entities`) are filtered out to avoid
/// redundant transmission.
///
/// # Example
///
/// ```ignore
/// fn model_model_demo() -> impl Bundle {
///     (Sequence, children![
///         (Name::new("User"), request_to_context()),
///         (
///             Name::new("Alice"),
///             ModelAction::new(OllamaProvider::default())
///                 .with_instructions("You are Alice, a helpful assistant."),
///         ),
///         (
///             Name::new("Bob"),
///             ModelAction::new(OllamaProvider::default())
///                 .with_instructions("You are Bob, a friendly conversationalist."),
///         ),
///         context_to_response()
///     ])
/// }
/// ```
#[derive(Component)]
#[component(on_add = on_add_model_action)]
pub struct ModelAction {
	/// The model provider (boxed for object safety).
	provider: BoxedModelProvider,
	/// The model name to use for requests.
	model: String,
	/// Whether to use streaming mode.
	stream: bool,
	/// System instructions to include with each request.
	instructions: Option<String>,
	/// Providers may track the last sent id
	previous_response_id: Option<String>,
	/// Track which item was sent last, for skipping sent
	/// items when a previous_response_id is used.
	last_item_sent: Option<ItemId>,
	partial_items: PartialItemMap,
}


impl std::fmt::Debug for ModelAction {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ModelAction")
			.field("model", &self.model)
			.field("stream", &self.stream)
			.field("instructions", &self.instructions)
			.field("previous_response_id", &self.previous_response_id)
			.field("last_item_sent", &self.last_item_sent)
			.finish_non_exhaustive()
	}
}

impl ModelAction {
	/// Creates a new model action with the given provider.
	///
	/// Uses the provider's default small model.
	pub fn new(provider: impl ModelProvider) -> Self {
		let model = provider.default_small_model();
		Self::with_model(provider, model)
	}

	/// Creates a new model action with a specific model name.
	pub fn with_model(
		provider: impl ModelProvider,
		model: impl Into<String>,
	) -> Self {
		Self {
			provider: Box::new(provider),
			model: model.into(),
			stream: false,
			instructions: None,
			partial_items: default(),
			previous_response_id: None,
			last_item_sent: None,
		}
	}


	pub fn last_item_sent(&self) -> Option<ItemId> { self.last_item_sent }
	pub fn previous_response_id(&self) -> Option<&str> {
		self.previous_response_id.as_deref()
	}

	/// Sets the model name.
	pub fn model(mut self, model: impl Into<String>) -> Self {
		self.model = model.into();
		self
	}

	/// Enables streaming mode.
	pub fn streaming(mut self) -> Self {
		self.stream = true;
		self
	}

	/// Sets whether streaming is enabled.
	pub fn with_stream(mut self, stream: bool) -> Self {
		self.stream = stream;
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

	/// Gets a reference to the provider.
	pub fn provider(&self) -> &dyn ModelProvider { self.provider.as_ref() }

	/// Gets the model name.
	pub fn model_name(&self) -> &str { &self.model }

	/// Builds a request body from the given input items.
	pub fn build_request(
		&mut self,
		context_map: &ContextMap,
		actor_id: ActorId,
		thread_id: ThreadId,
	) -> Result<openresponses::RequestBody> {
		let input = self.build_input(context_map, actor_id, thread_id)?;
		let tools = vec![];

		let mut body = openresponses::RequestBody::new(&self.model)
			.with_input(input)
			.with_tools(tools);

		if let Some(prev_res) = &self.previous_response_id {
			body = body.with_previous_response_id(prev_res);
		}

		if let Some(instructions) = &self.instructions {
			body = body.with_instructions(instructions);
		}

		if self.stream {
			body = body.with_stream(true);
		}

		body.xok()
	}
	fn build_input(
		&mut self,
		context_map: &ContextMap,
		actor_id: ActorId,
		thread_id: ThreadId,
	) -> Result<openresponses::request::Input> {
		// only provide the last item sent if last response was cached
		let last_item_sent = if self.previous_response_id.is_some() {
			self.last_item_sent
		} else {
			None
		};

		// collect input
		let input = self.partial_items.build_input(
			context_map,
			actor_id,
			thread_id,
			last_item_sent,
		)?;

		// remember the most recent item
		self.last_item_sent = context_map
			.threads()
			.get(thread_id)?
			.items()
			.last()
			.cloned();

		input.xok()
	}

	fn parse_response_body(
		&mut self,
		context_query: &mut ContextQuery,
		actor_id: ActorId,
		response: openresponses::ResponseBody,
		status: ItemStatus,
	) -> Result {
		// if let Some(prev_response_id) = response.previous_response_id {
		// 	self.previous_response_id = Some(prev_response_id);
		// }
		self.previous_response_id = Some(response.id.clone());

		let partial_items =
			PartialItem::from_output_items(response.output, status);

		self.handle_partial_items(context_query, actor_id, partial_items)?;

		match status {
			ItemStatus::Completed => {
				context_query.response_complete(response.id, false)
			}
			ItemStatus::Interrupted => {
				context_query.response_complete(response.id, true)
			}
			ItemStatus::InProgress => {}
		}

		Ok(())
	}

	fn handle_partial_items(
		&mut self,
		context_query: &mut ContextQuery,
		owner: ActorId,
		items: impl IntoIterator<Item = PartialItem>,
	) -> Result {
		let changes = self.partial_items.apply_items(
			context_query.items_mut(),
			owner,
			items,
		)?;
		context_query.handle_item_changes(changes)?;
		Ok(())
	}

	fn handle_event(
		&mut self,
		context_query: &mut ContextQuery,
		owner: ActorId,
		event: openresponses::StreamingEvent,
	) -> Result<ControlFlow<()>> {
		use openresponses::StreamingEvent::*;
		trace!("Streaming Event: {:#?}", event);
		match event {
			ResponseCreated(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(
					context_query,
					owner,
					ev.response,
					ItemStatus::InProgress,
				)?;
			}
			ResponseQueued(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(
					context_query,
					owner,
					ev.response,
					ItemStatus::InProgress,
				)?;
			}
			ResponseInProgress(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(
					context_query,
					owner,
					ev.response,
					ItemStatus::InProgress,
				)?;
			}
			ResponseCompleted(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(
					context_query,
					owner,
					ev.response,
					ItemStatus::Completed,
				)?;
				return ControlFlow::Break(()).xok();
			}
			ResponseFailed(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(
					context_query,
					owner,
					ev.response,
					ItemStatus::Interrupted,
				)?;
				return ControlFlow::Break(()).xok();
			}
			ResponseIncomplete(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(
					context_query,
					owner,
					ev.response,
					ItemStatus::Interrupted,
				)?;
				return ControlFlow::Break(()).xok();
			}
			OutputItemAdded(item_added) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem::from_output_items(
						item_added.item,
						ItemStatus::InProgress,
					),
				)?;
			}
			OutputItemDone(item_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem::from_output_items(
						item_done.item,
						ItemStatus::Completed,
					),
				)?;
			}
			ContentPartAdded(part_added) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: part_added.item_id,
							content_index: part_added.content_index,
						},
						status: ItemStatus::InProgress,
						content: PartialContent::ContentPart(part_added.part),
					}
					.xsome(),
				)?;
			}
			ContentPartDone(part_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: part_done.item_id,
							content_index: part_done.content_index,
						},
						status: ItemStatus::Completed,
						content: PartialContent::ContentPart(part_done.part),
					}
					.xsome(),
				)?;
			}
			OutputTextDelta(text_delta) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: text_delta.item_id,
							content_index: text_delta.content_index,
						},
						status: ItemStatus::InProgress,
						content: PartialContent::Delta(text_delta.delta),
					}
					.xsome(),
				)?;
			}
			OutputTextDone(text_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: text_done.item_id,
							content_index: text_done.content_index,
						},
						status: ItemStatus::Completed,
						content: PartialContent::TextDone {
							text: text_done.text,
							logprobs: text_done.logprobs,
						},
					}
					.xsome(),
				)?;
			}
			OutputTextAnnotationAdded(annotation_added) => {
				if let Some(annotation) = annotation_added.annotation {
					self.handle_partial_items(
						context_query,
						owner,
						PartialItem {
							key: PartialItemKey::Content {
								responses_id: annotation_added.item_id,
								content_index: annotation_added.content_index,
							},
							status: ItemStatus::InProgress,
							content: PartialContent::AnnotationAdded {
								annotation,
								annotation_index: annotation_added
									.annotation_index,
							},
						}
						.xsome(),
					)?;
				} else {
					// no annotation, nothing to do
				}
			}
			RefusalDelta(refusal_delta) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: refusal_delta.item_id,
							content_index: refusal_delta.content_index,
						},
						status: ItemStatus::InProgress,
						content: PartialContent::Delta(refusal_delta.delta),
					}
					.xsome(),
				)?;
			}
			RefusalDone(refusal_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: refusal_done.item_id,
							content_index: refusal_done.content_index,
						},
						status: ItemStatus::Completed,
						content: PartialContent::RefusalDone {
							refusal: refusal_done.refusal,
						},
					}
					.xsome(),
				)?;
			}
			ReasoningDelta(reasoning_delta) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: reasoning_delta.item_id,
							content_index: reasoning_delta.content_index,
						},
						status: ItemStatus::InProgress,
						content: PartialContent::Delta(reasoning_delta.delta),
					}
					.xsome(),
				)?;
			}
			ReasoningDone(reasoning_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Content {
							responses_id: reasoning_done.item_id,
							content_index: reasoning_done.content_index,
						},
						status: ItemStatus::Completed,
						content: PartialContent::ReasoningDone {
							content: reasoning_done.text,
						},
					}
					.xsome(),
				)?;
			}
			ReasoningSummaryTextDelta(summary_delta) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::ReasoningSummary {
							responses_id: summary_delta.item_id,
							summary_index: summary_delta
								.summary_index
								.unwrap_or(0),
						},
						status: ItemStatus::InProgress,
						content: PartialContent::Delta(summary_delta.delta),
					}
					.xsome(),
				)?;
			}
			ReasoningSummaryTextDone(summary_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::ReasoningSummary {
							responses_id: summary_done.item_id,
							summary_index: summary_done
								.summary_index
								.unwrap_or(0),
						},
						status: ItemStatus::Completed,
						content: PartialContent::ReasoningSummary(
							summary_done.text,
						),
					}
					.xsome(),
				)?;
			}
			ReasoningSummaryPartAdded(summary_added) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::ReasoningSummary {
							responses_id: summary_added.item_id,
							summary_index: summary_added
								.summary_index
								.unwrap_or(0),
						},
						status: ItemStatus::InProgress,
						content: PartialContent::ContentPart(
							summary_added.part,
						),
					}
					.xsome(),
				)?;
			}
			ReasoningSummaryPartDone(summary_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::ReasoningSummary {
							responses_id: summary_done.item_id,
							summary_index: summary_done
								.summary_index
								.unwrap_or(0),
						},
						status: ItemStatus::Completed,
						content: PartialContent::ContentPart(summary_done.part),
					}
					.xsome(),
				)?;
			}
			FunctionCallArgumentsDelta(arguments_delta) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Single {
							responses_id: arguments_delta.item_id,
						},
						status: ItemStatus::InProgress,
						content: PartialContent::Delta(arguments_delta.delta),
					}
					.xsome(),
				)?;
			}
			FunctionCallArgumentsDone(arguments_done) => {
				self.handle_partial_items(
					context_query,
					owner,
					PartialItem {
						key: PartialItemKey::Single {
							responses_id: arguments_done.item_id,
						},
						status: ItemStatus::Completed,
						content: PartialContent::FunctionCallArgumentsDone(
							arguments_done.arguments,
						),
					}
					.xsome(),
				)?;
			}
			Error(error) => {
				bevybail!("Model streaming error: {:?}", error.error);
			}
		}

		ControlFlow::Continue(()).xok()
	}
}

fn on_add_model_action(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(call_model.into_tool());
}

#[tool]
pub async fn call_model(input: AsyncToolIn<()>) -> Result {
	let entity = input.caller.id();
	let world = input.caller.world();
	let (provider, request) =
		world.run_system_cached_with(build_request, entity).await?;
	if request.stream == Some(true) {
		let mut stream = provider.stream(request).await?;
		while let Some(event) = stream.next().await {
			let event = event?;
			match world
				.run_system_cached_with::<_, ControlFlow<()>, _, _>(
					handle_event,
					(entity, event),
				)
				.await?
			{
				ControlFlow::Continue(_) => {}
				ControlFlow::Break(_) => {
					break;
				}
			}
		}
	} else {
		let response = provider.send(request).await?;
		world
			.run_system_cached_with::<_, (), _, _>(
				handle_response,
				(entity, response),
			)
			.await?;
	}

	Ok(())
}

fn build_request(
	In(entity): In<Entity>,
	context_map: Res<ContextMap>,
	mut query: Query<(&ActorId, &ThreadId, &mut ModelAction)>,
) -> Result<(Box<dyn ModelProvider>, openresponses::RequestBody)> {
	let (actor_id, thread_id, mut model_action) = query.get_mut(entity)?;
	let request =
		model_action.build_request(&context_map, *actor_id, *thread_id)?;

	Ok((model_action.provider().box_clone(), request))
}

fn handle_response(
	In((entity, response)): In<(Entity, openresponses::ResponseBody)>,
	mut context_query: ContextQuery,
	mut query: Query<(&ActorId, &mut ModelAction)>,
) -> Result {
	let (actor_id, mut model_action) = query.get_mut(entity)?;
	model_action.parse_response_body(
		&mut context_query,
		*actor_id,
		response,
		ItemStatus::Completed,
	)
}
fn handle_event(
	In((entity, event)): In<(Entity, openresponses::StreamingEvent)>,
	mut context_query: ContextQuery,
	mut query: Query<(&ActorId, &mut ModelAction)>,
) -> Result<ControlFlow<()>> {
	let (actor_id, mut model_action) = query.get_mut(entity)?;
	model_action.handle_event(&mut context_query, *actor_id, event)
}


// 			commands.run_local(async move |world| -> Result {
// 				if let Some(true) = body.stream {
// 					// Streaming mode
// 					let mut stream = provider.stream(body).await?;

// 					let mut spawner = StreamingContextSpawner::new(
// 						world.clone(),
// 						agent,
// 						action,
// 					);

// 					// Get BodyStream from agent if available (for HTTP streaming)
// 					if let Ok(body_stream) =
// 						world.entity(agent).get_cloned::<BodyStream>().await
// 					{
// 						spawner = spawner.with_body_stream(body_stream);
// 					}

// 					while let Some(event) = stream.next().await {
// 						let event = event?;
// 						match spawner.handle_event(&event).await? {
// 							std::ops::ControlFlow::Continue(_) => {}
// 							std::ops::ControlFlow::Break(_) => break,
// 						}
// 					}

// 					// Update model action state
// 					let response_id =
// 						spawner.response_id().map(|s| s.to_string());
// 					world
// 						.entity(action)
// 						.with_then(move |mut entity| {
// 							if let Some(mut model_action) =
// 								entity.get_mut::<ModelAction>()
// 							{
// 								// Restore the provider
// 								model_action.provider = provider;
// 								// Update previous_response_id
// 								if let Some(id) = response_id {
// 									model_action.previous_response_id =
// 										Some(id);
// 								}
// 							}
// 						})
// 						.await;
