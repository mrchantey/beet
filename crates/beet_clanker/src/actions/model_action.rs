//! Model action component for AI agent interactions.
//!
//! The [`ModelAction`] component configures how an action entity interacts
//! with an AI model. When added to an entity, it automatically inserts the
//! request behavior via an `on_add` hook.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::BodyStream;


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
	pub stream: bool,
	/// System instructions to include with each request.
	pub instructions: Option<String>,
	/// The previous response ID for multi-turn conversations.
	/// This is updated automatically after each request.
	pub previous_response_id: Option<String>,
	/// Tracks context entities that have already been sent to the model.
	/// Used to filter out redundant context when `previous_response_id` is set.
	sent_context_entities: HashSet<Entity>,
}

impl std::fmt::Debug for ModelAction {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ModelAction")
			.field("model", &self.model)
			.field("stream", &self.stream)
			.field("instructions", &self.instructions)
			.field("previous_response_id", &self.previous_response_id)
			.field("sent_context_count", &self.sent_context_entities.len())
			.finish_non_exhaustive()
	}
}

impl ModelAction {
	/// Creates a new model action with the given provider.
	///
	/// Uses the provider's default small model.
	pub fn new(provider: impl ModelProvider) -> Self {
		let model = provider.default_small_model().to_string();
		Self {
			provider: Box::new(provider),
			model,
			stream: false,
			instructions: None,
			previous_response_id: None,
			sent_context_entities: HashSet::default(),
		}
	}

	pub fn sent_context_entities(&self) -> &HashSet<Entity> {
		&self.sent_context_entities
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
			previous_response_id: None,
			sent_context_entities: HashSet::default(),
		}
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

	/// Takes the provider out of this action, leaving a placeholder.
	/// Used to move the provider into an async context.
	pub fn take_provider(&mut self) -> BoxedModelProvider {
		std::mem::replace(&mut self.provider, Box::new(PlaceholderProvider))
	}

	/// Gets the model name.
	pub fn model_name(&self) -> &str { &self.model }

	/// Builds a request body from the given input items.
	pub fn build_request(
		&self,
		input_items: Vec<openresponses::request::InputItem>,
		tools: Vec<openresponses::FunctionToolParam>,
	) -> openresponses::RequestBody {
		let mut body = openresponses::RequestBody::new(&self.model)
			.with_input_items(input_items)
			.with_tools(tools);

		if let Some(prev_id) = &self.previous_response_id {
			body = body.with_previous_response_id(prev_id);
		}

		if let Some(instructions) = &self.instructions {
			body = body.with_instructions(instructions);
		}

		if self.stream {
			body = body.with_stream(true);
		}

		body
	}
	/// Collects input items from context, filtering out already-sent entities if
	/// the model action has a previous_response_id.
	///
	/// Returns the input items
	fn collect_filtered_input_items(
		&mut self,
		query: &ContextQuery,
		action: Entity,
	) -> Result<Vec<openresponses::request::InputItem>> {
		let some_context_cached = self.previous_response_id.is_some();


		// Collect input items only for context we havent sent yet
		let input_items = query.collect_input_items(action, |entity| {
			// pass if this provider isnt caching at all
			!some_context_cached ||
			// pass if this entity is missing from sent context
			!self.sent_context_entities.contains(entity)
		})?;

		self.mark_context_sent(query, action);

		input_items.xok()
	}
	/// Mark all existing context entities as sent so they won't be resent.
	fn mark_context_sent(&mut self, query: &ContextQuery, action: Entity) {
		let entities = query
			.contexts
			.get(action)
			.map(|cx| (*cx).clone())
			.unwrap_or_default();

		self.sent_context_entities.extend(entities);
	}
}

/// A placeholder provider used when the real provider has been taken.
/// All methods panic if called, as the provider should be restored before use.
// TODO this pattern sucks, i think if we can use a Sync BoxedFuture it wont be nessecary
struct PlaceholderProvider;

impl ModelProvider for PlaceholderProvider {
	fn provider_slug(&self) -> &'static str { "placeholder" }
	fn default_small_model(&self) -> &'static str { "" }
	fn default_tool_model(&self) -> &'static str { "" }
	fn default_large_model(&self) -> &'static str { "" }

	fn send(
		&self,
		_request: openresponses::RequestBody,
	) -> bevy::tasks::BoxedFuture<'_, Result<openresponses::ResponseBody>> {
		Box::pin(async {
			bevybail!(
				"PlaceholderProvider: provider was taken and not restored"
			)
		})
	}

	fn stream(
		&self,
		_request: openresponses::RequestBody,
	) -> bevy::tasks::BoxedFuture<'_, Result<StreamingEventStream>> {
		Box::pin(async {
			bevybail!(
				"PlaceholderProvider: provider was taken and not restored"
			)
		})
	}
}

fn on_add_model_action(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.insert(model_action_request());
}


/// Creates the model action request behavior.
///
/// This is automatically inserted by the `ModelAction` component's `on_add` hook.
/// It observes `GetOutcome` events and sends context to the configured model.
pub fn model_action_request() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 context_query: ContextQuery,
		 tool_query: ToolQuery,
		 mut model_query: Query<&mut ModelAction>,
		 agents: AgentQuery,
		 mut commands: AsyncCommands|
		 -> Result {
			let action = ev.target();

			// ModelAction is required
			let mut model_action = model_query.get_mut(action)?;

			let agent = agents.entity(action);

			// Collect context into input items
			let input_items = model_action
				.collect_filtered_input_items(&context_query, action)?;

			if input_items.is_empty() {
				bevybail!("No context to send to AI agent");
			}

			let tools = tool_query.collect_tools(action)?;

			// Build request body
			let body = model_action.build_request(input_items, tools);

			// Take ownership of the provider for the async block
			let provider = model_action.take_provider();


			commands.run_local(async move |world| -> Result {
				if let Some(true) = body.stream {
					// Streaming mode
					let mut stream = provider.stream(body).await?;

					let mut spawner = StreamingContextSpawner::new(
						world.clone(),
						agent,
						action,
					);

					// Get BodyStream from agent if available (for HTTP streaming)
					if let Ok(body_stream) =
						world.entity(agent).get_cloned::<BodyStream>().await
					{
						spawner = spawner.with_body_stream(body_stream);
					}

					while let Some(event) = stream.next().await {
						let event = event?;
						match spawner.handle_event(&event).await? {
							std::ops::ControlFlow::Continue(_) => {}
							std::ops::ControlFlow::Break(_) => break,
						}
					}

					// Update model action state
					let response_id =
						spawner.response_id().map(|s| s.to_string());
					world
						.entity(action)
						.with_then(move |mut entity| {
							if let Some(mut model_action) =
								entity.get_mut::<ModelAction>()
							{
								// Restore the provider
								model_action.provider = provider;
								// Update previous_response_id
								if let Some(id) = response_id {
									model_action.previous_response_id =
										Some(id);
								}
							}
						})
						.await;
				} else {
					// Non-streaming mode
					let response = provider.send(body).await?;

					let response_id = response.id.clone();
					// Spawn context entities from the response
					context_spawner::spawn_response_context(
						&world, agent, action, response,
					)
					.await?;

					// Update model action state
					world
						.entity(action)
						.with_then(move |mut entity| {
							if let Some(mut model_action) =
								entity.get_mut::<ModelAction>()
							{
								// Restore the provider
								model_action.provider = provider;
								// Update previous_response_id
								model_action.previous_response_id =
									Some(response_id);
							}
						})
						.await;
				}

				world
					.entity(action)
					.trigger_target_then(Outcome::Pass)
					.await;
				Ok(())
			});
			Ok(())
		},
	)
}
