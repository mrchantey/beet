//! Oneshot AI agent action for single request-response interactions.
//!
//! This module provides a flow action that sends context to an AI model
//! and receives a response, supporting both streaming and non-streaming modes.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


/// Creates a oneshot AI agent behavior tree.
///
/// This behavior:
/// 1. Loads request data into context
/// 2. Sends context to the AI model
/// 3. Returns the response
pub fn oneshot() -> impl Bundle {
	(Sequence, children![
		request_to_context(),
		model_action_request(),
		context_to_response()
	])
}


fn request_to_context() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 agent_query: AgentQuery<&RequestMeta>,
		 mut commands: AsyncCommands|
		 -> Result {
			let action = ev.target();
			let req_meta = agent_query.get(action)?;
			let query = req_meta.path().join(" ");

			let agent = agent_query.entity(action);

			commands.run_local(async move |world| -> Result {
				spawn_user_context(&world, agent, action, query).await;
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


fn context_to_response() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 query: ContextQuery,
		 roles: Query<&ContextRole>,
		 agents: AgentQuery,
		 mut commands: Commands|
		 -> Result {
			let action = ev.target();
			let agent = agents.entity(action);

			// Collect assistant responses from context
			let mut response_parts = Vec::new();
			if let Ok(context) = query.contexts.get(action) {
				for ctx_entity in context.iter() {
					// Only include assistant responses
					let is_assistant = roles
						.get(ctx_entity)
						.map(|role| *role == ContextRole::Assistant)
						.unwrap_or(false);

					if is_assistant {
						if let Ok(text) = query.text_contexts.get(ctx_entity) {
							if !text.0.is_empty() {
								response_parts.push(text.0.clone());
							}
						}
					}
				}
			}

			let response_text = response_parts.join("\n");
			commands
				.entity(agent)
				.insert(Response::ok().with_body(response_text));
			commands.entity(action).trigger_target(Outcome::Pass);
			Ok(())
		},
	)
}


/// Configuration for a model action (an action that interacts with a model).
///
/// This is stored on the action entity itself, allowing multiple actions
/// in the same behavior tree to have different configurations. This enables
/// multi-agent conversations where each agent has its own model settings.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct ModelActionConfig {
	/// Whether to use streaming mode.
	pub stream: bool,
	/// The previous response ID for multi-turn conversations.
	pub previous_response_id: Option<String>,
	/// System instructions to include with each request.
	/// Use this to describe the model's role in the conversation.
	pub instructions: Option<String>,
}

impl ModelActionConfig {
	/// Creates a new streaming configuration.
	pub fn streaming() -> Self {
		Self {
			stream: true,
			..default()
		}
	}

	/// Sets the instructions for this model action.
	pub fn with_instructions(
		mut self,
		instructions: impl Into<String>,
	) -> Self {
		self.instructions = Some(instructions.into());
		self
	}
}


fn model_action_request() -> impl Bundle {
	OnSpawn::observe(
		|ev: On<GetOutcome>,
		 query: ContextQuery,
		 config_query: Query<Option<&ModelActionConfig>>,
		 agents: AgentQuery,
		 mut commands: AsyncCommands|
		 -> Result {
			let action = ev.target();
			let agent = agents.entity(action);

			// Collect context into input items
			let input_items = query.to_input_items(action);

			if input_items.is_empty() {
				bevybail!("No context to send to AI agent");
			}

			// Get configuration
			let config = config_query
				.get(action)
				.ok()
				.flatten()
				.cloned()
				.unwrap_or_default();

			commands.run_local(async move |world| -> Result {
				let mut provider = OllamaProvider::default();

				let mut body = openresponses::RequestBody::new(
					provider.default_small_model(),
				)
				.with_input_items(input_items);

				// Set previous response ID if available
				if let Some(prev_id) = &config.previous_response_id {
					body = body.with_previous_response_id(prev_id);
				}

				// Set instructions if provided
				if let Some(instructions) = &config.instructions {
					body = body.with_instructions(instructions);
				}

				if config.stream {
					// Streaming mode
					body = body.with_stream(true);

					let mut stream = provider.stream(body).await?;

					let mut spawner =
						ContextSpawner::new(world.clone(), agent, action);

					while let Some(event) = stream.next().await {
						let event = event?;
						let done = spawner.handle_event(&event).await?;
						if done {
							break;
						}
					}

					// Update previous_response_id on the action if we got one
					if let Some(response_id) = spawner.response_id() {
						let response_id = response_id.to_string();
						world
							.entity(action)
							.with_then(move |mut entity| {
								if let Some(mut config) =
									entity.get_mut::<ModelActionConfig>()
								{
									config.previous_response_id =
										Some(response_id);
								} else {
									entity.insert(ModelActionConfig {
										previous_response_id: Some(response_id),
										stream: true,
										instructions: None,
									});
								}
							})
							.await;
					}
				} else {
					// Non-streaming mode
					let response = provider.send(body).await?;

					// Spawn context entities from the response
					spawn_response_context(&world, agent, action, &response)
						.await?;

					// Update previous_response_id
					let response_id = response.id.clone();
					world
						.entity(action)
						.with_then(move |mut entity| {
							if let Some(mut config) =
								entity.get_mut::<ModelActionConfig>()
							{
								config.previous_response_id = Some(response_id);
							} else {
								entity.insert(ModelActionConfig {
									previous_response_id: Some(response_id),
									stream: false,
									instructions: None,
								});
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


/// Creates a streaming oneshot AI agent behavior tree.
pub fn oneshot_streaming() -> impl Bundle {
	(Sequence, ModelActionConfig::streaming(), children![
		request_to_context(),
		model_action_request(),
		context_to_response()
	])
}


#[cfg(test)]
mod test {
	use beet_net::prelude::*;

	use super::*;


	#[beet_core::test(timeout_ms = 15_000)]
	async fn non_streaming() {
		FlowAgentPlugin::world()
			.spawn(flow_exchange(oneshot))
			.exchange_str(
				Request::from_cli_str(
					"whats the capital of thailand? one word, capital first letter, no fullstop",
				)
				.unwrap(),
			)
			.await
			.xpect_eq("Bangkok");
	}

	#[beet_core::test(timeout_ms = 15_000)]
	async fn streaming() {
		FlowAgentPlugin::world()
			.spawn(flow_exchange(oneshot_streaming))
			.exchange_str(
				Request::from_cli_str(
					"whats the capital of japan? one word, capital first letter, no fullstop",
				)
				.unwrap(),
			)
			.await
			.xpect_eq("Tokyo");
	}


	/// Test demonstrating two models conversing with shared context.
	///
	/// This test verifies:
	/// 1. Context ownership tracking via `ContextMeta::created_by`
	/// 2. Role determination: "my" context = Assistant, "others'" = User
	/// 3. Name prefixing for multi-agent identification
	#[beet_core::test(timeout_ms = 60_000)]
	async fn model_to_model_conversation() {
		// Use run_async_local_then to properly handle async in test context
		FlowAgentPlugin::world()
			.run_async_local_then(|world| async move {
				// Spawn an agent entity that both model actions will share
				let agent = world
					.spawn_then((Name::new("Shared Agent"),))
					.await
					.id();

				// Create the initial user context (a question for the first model)
				// We use Entity::PLACEHOLDER as the action since this is user input
				spawn_user_context(
					&world,
					agent,
					Entity::PLACEHOLDER,
					"What is 2 + 2? Reply with just the number.",
				)
				.await;

				// First model (Alice) answers the question
				run_model_action(
					&world,
					agent,
					"Alice",
					"You are Alice. Answer questions directly and concisely with just the answer.",
				)
				.await
				.unwrap();

				// Second model (Bob) responds to Alice's answer
				run_model_action(
					&world,
					agent,
					"Bob",
					"You are Bob. When you see a number answer from someone, respond with exactly 'Confirmed: X' where X is the number you saw.",
				)
				.await
				.unwrap();

				// Collect all text contexts
				let texts: Vec<String> = world
					.with_then(|world| {
						let mut query = world.query::<&TextContext>();
						query.iter(world).map(|ctx| ctx.0.clone()).collect()
					})
					.await;

				// Should have at least 3 contexts: user question, Alice's answer, Bob's confirmation
				texts.len().xpect_greater_or_equal_to(3);

				// The last context should be from Bob and contain "Confirmed" and "4"
				let bob_response = texts.last().cloned().unwrap_or_default();

				bob_response
					.to_lowercase()
					.contains("confirmed")
					.xpect_true();
				bob_response.contains("4").xpect_true();
			})
			.await;
	}
}


/// Runs a model action with the given instructions and waits for completion.
///
/// This is a helper for multi-agent tests that creates an action entity,
/// sends context to the model, and spawns the response back into context.
#[allow(dead_code)]
async fn run_model_action(
	world: &AsyncWorld,
	agent: Entity,
	name: &str,
	instructions: &str,
) -> Result<()> {
	// Create a unique action entity for this model interaction
	let action = world.spawn_then((Name::new(name.to_string()),)).await.id();

	// Build input items from context, treating this action as "self"
	let input_items: Vec<openresponses::request::InputItem> = world
		.with_then(move |world| {
			let mut items = Vec::new();

			// Query context entities associated with this agent
			let mut query =
				world
					.query::<(&ContextOf, Option<&TextContext>, Option<&ContextMeta>)>(
					);

			for (context_of, text, meta) in query.iter(world) {
				if **context_of != agent {
					continue;
				}

				// Determine role based on who created this context
				let created_by = meta.and_then(|m| m.created_by);
				let effective_role = if created_by == Some(action) {
					openresponses::MessageRole::Assistant
				} else {
					openresponses::MessageRole::User
				};

				if let Some(text) = text {
					// Prefix with creator's name if not self
					let text_content = if created_by != Some(action) {
						if let Some(creator) = created_by {
							if let Some(creator_name) =
								world.get::<Name>(creator)
							{
								format!("{} > {}", creator_name, text.0)
							} else {
								text.0.clone()
							}
						} else {
							text.0.clone()
						}
					} else {
						text.0.clone()
					};

					items.push(openresponses::request::InputItem::Message(
						openresponses::request::MessageParam {
							id: None,
							role: effective_role,
							content:
								openresponses::request::MessageContent::Text(
									text_content,
								),
							status: None,
						},
					));
				}
			}

			items
		})
		.await;

	if input_items.is_empty() {
		bevybail!("No context to send to model");
	}

	// Send to model
	let mut provider = OllamaProvider::default();
	let body = openresponses::RequestBody::new(provider.default_small_model())
		.with_input_items(input_items)
		.with_instructions(instructions)
		.with_stream(true);

	let mut stream = provider.stream(body).await?;
	let mut spawner = ContextSpawner::new(world.clone(), agent, action);

	while let Some(event) = stream.next().await {
		let event = event?;
		let done = spawner.handle_event(&event).await?;
		if done {
			break;
		}
	}

	Ok(())
}
