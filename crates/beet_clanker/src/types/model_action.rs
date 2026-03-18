//! Model action component for AI agent interactions.
//!
//! The [`ModelAction`] component configures how an action entity interacts
//! with an AI model. When added to an entity, it automatically inserts the
//! request behavior via an `on_add` hook.

use std::ops::ControlFlow;

use crate::openresponses::StreamingEvent;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_tool::prelude::*;

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
	item_mapper: ItemMapper,
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
			item_mapper: default(),
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
		let input = self.item_mapper.build_input(
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
	) -> Result {
		if let Some(prev_response_id) = response.previous_response_id {
			self.previous_response_id = Some(prev_response_id);
		}
		let items = self.item_mapper.parse_output(actor_id, response.output)?;
		context_query.add_items(items)?;

		Ok(())
	}

	fn handle_event(
		&mut self,
		context_query: &mut ContextQuery,
		actor_id: ActorId,
		event: StreamingEvent,
	) -> Result<ControlFlow<()>> {
		use openresponses::StreamingEvent::*;
		// info!("Handling streaming event: {:#?}", event);
		match event {
			ResponseCreated(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(context_query, actor_id, ev.response)?;
			}
			ResponseQueued(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(context_query, actor_id, ev.response)?;
			}
			ResponseInProgress(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(context_query, actor_id, ev.response)?;
			}
			ResponseCompleted(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(context_query, actor_id, ev.response)?;
				return ControlFlow::Break(()).xok();
			}
			ResponseFailed(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(context_query, actor_id, ev.response)?;
				return ControlFlow::Break(()).xok();
			}
			ResponseIncomplete(ev) => {
				// usually empty but parse anyway
				self.parse_response_body(context_query, actor_id, ev.response)?;
				return ControlFlow::Break(()).xok();
			}
			OutputItemAdded(_output_item_added_event) => todo!(),
			OutputItemDone(_output_item_done_event) => todo!(),
			ContentPartAdded(_content_part_added_event) => todo!(),
			ContentPartDone(_content_part_done_event) => todo!(),
			OutputTextDelta(_output_text_delta_event) => todo!(),
			OutputTextDone(_output_text_done_event) => todo!(),
			OutputTextAnnotationAdded(_output_text_annotation_added_event) => {
				todo!()
			}
			RefusalDelta(_refusal_delta_event) => todo!(),
			RefusalDone(_refusal_done_event) => todo!(),
			ReasoningDelta(_reasoning_delta_event) => todo!(),
			ReasoningDone(_reasoning_done_event) => todo!(),
			ReasoningSummaryTextDelta(_reasoning_summary_text_delta_event) => {
				todo!()
			}
			ReasoningSummaryTextDone(_reasoning_summary_text_done_event) => {
				todo!()
			}
			ReasoningSummaryPartAdded(_reasoning_summary_part_added_event) => {
				todo!()
			}
			ReasoningSummaryPartDone(_reasoning_summary_part_done_event) => {
				todo!()
			}
			FunctionCallArgumentsDelta(
				_function_call_arguments_delta_event,
			) => {
				todo!()
			}
			FunctionCallArgumentsDone(_function_call_arguments_done_event) => {
				todo!()
			}
			Error(_error_event) => todo!(),
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
	model_action.parse_response_body(&mut context_query, *actor_id, response)
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
