//! Model action component for AI agent interactions.
//!
//! The [`ModelAction`] component configures how an action entity interacts
//! with an AI model. When added to an entity, it automatically inserts the
//! request behavior via an `on_add` hook.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


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
}

impl std::fmt::Debug for ModelAction {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ModelAction")
			.field("model", &self.model)
			.field("stream", &self.stream)
			.field("instructions", &self.instructions)
			.field("previous_response_id", &self.previous_response_id)
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
		}
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

	/// Gets the model name.
	pub fn model_name(&self) -> &str { &self.model }

	/// Builds a request body from the given input items.
	pub fn build_request(
		&self,
		input_items: Vec<openresponses::request::InputItem>,
	) -> openresponses::RequestBody {
		let mut body = openresponses::RequestBody::new(&self.model)
			.with_input_items(input_items);

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
		 query: ContextQuery,
		 mut model_query: Query<&mut ModelAction>,
		 agents: AgentQuery,
		 mut commands: AsyncCommands|
		 -> Result {
			let action = ev.target();

			// ModelAction is required
			let model_action = model_query.get_mut(action)?;

			let agent = agents.entity(action);

			// Collect context into input items
			let input_items = query.collect_input_items(action);

			if input_items.is_empty() {
				bevybail!("No context to send to AI agent");
			}

			// Build request body
			let body = model_action.build_request(input_items);
			let stream = model_action.stream;

			// Clone what we need for the async block
			// We need to get the provider out for the async block
			// Since we can't move the provider out, we'll need to handle this differently
			// For now, we'll create a new provider in the async block based on the slug
			let provider_slug =
				model_action.provider().provider_slug().to_string();

			commands.run_local(async move |world| -> Result {
				// Create provider based on slug
				// This is a temporary approach until we have a provider registry
				let provider: BoxedModelProvider = match provider_slug.as_str()
				{
					"ollama" => Box::new(OllamaProvider::default()),
					"openai" => Box::new(OpenAIProvider::default()),
					_ => bevybail!("Unknown provider: {}", provider_slug),
				};

				if stream {
					// Streaming mode
					let mut stream = provider.stream(body).await?;

					let mut spawner = StreamingContextSpawner::new(
						world.clone(),
						agent,
						action,
					);

					while let Some(event) = stream.next().await {
						let event = event?;
						match spawner.handle_event(&event).await? {
							std::ops::ControlFlow::Continue(_) => {}
							std::ops::ControlFlow::Break(_) => break,
						}
					}

					// Update previous_response_id on the action if we got one
					if let Some(response_id) = spawner.response_id() {
						let response_id = response_id.to_string();
						world
							.entity(action)
							.with_then(move |mut entity| {
								if let Some(mut model_action) =
									entity.get_mut::<ModelAction>()
								{
									model_action.previous_response_id =
										Some(response_id);
								}
							})
							.await;
					}
				} else {
					// Non-streaming mode
					let response = provider.send(body).await?;

					let response_id = response.id.clone();
					// Spawn context entities from the response
					context_spawner::spawn_response_context(
						&world, agent, action, response,
					)
					.await?;

					// Update previous_response_id
					world
						.entity(action)
						.with_then(move |mut entity| {
							if let Some(mut model_action) =
								entity.get_mut::<ModelAction>()
							{
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
