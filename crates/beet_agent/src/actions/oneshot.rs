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
///
/// Note: This uses the default Ollama provider. For custom providers,
/// manually construct the behavior tree with a configured [`ModelAction`].
pub fn oneshot() -> impl Bundle {
	(Sequence, children![
		request_to_context(),
		ModelAction::new(OllamaProvider::default()),
		context_to_response()
	])
}


/// Creates a streaming oneshot AI agent behavior tree.
///
/// Note: This uses the default Ollama provider. For custom providers,
/// manually construct the behavior tree with a configured [`ModelAction`].
pub fn oneshot_streaming() -> impl Bundle {
	(Sequence, children![
		request_to_context(),
		ModelAction::new(OllamaProvider::default()).streaming(),
		context_to_response()
	])
}


/// Loads request data into context.
///
/// This action reads the request from `RequestMeta` and spawns it as
/// user context for the AI model to process.
pub fn request_to_context() -> impl Bundle {
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
				context_spawner::spawn_user_context(
					&world, agent, action, query,
				)
				.await;
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


/// Converts context to a response.
///
/// This action collects assistant responses from context and assembles
/// them into a `Response` on the agent entity.
pub fn context_to_response() -> impl Bundle {
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
	/// 1. Context ownership tracking via `ContextMeta::owner`
	/// 2. Role determination: "my" context = Assistant, "others'" = User
	/// 3. Name prefixing for multi-agent identification
	#[beet_core::test(timeout_ms = 60_000)]
	async fn model_to_model_conversation() {
		FlowAgentPlugin::world()
			.run_async_local_then(|world| async move {
				// Spawn an agent entity that both model actions will share
				let agent = world
					.spawn_then((Name::new("Shared Agent"),))
					.await
					.id();

				// Create the initial user context (a question for the first model)
				// We use Entity::PLACEHOLDER as the action since this is user input
				context_spawner::spawn_user_context(
					&world,
					agent,
					Entity::PLACEHOLDER,
					"What is 2 + 2? Reply with just the number.",
				)
				.await;

				// First model (Alice) answers the question
				run_model_with_context(
					&world,
					agent,
					"Alice",
					"You are Alice. Answer questions directly and concisely with just the answer.",
				)
				.await
				.unwrap();

				// Second model (Bob) responds to Alice's answer
				run_model_with_context(
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


	/// Helper for multi-agent tests that creates an action entity and runs it
	/// using the standard ModelAction pattern.
	async fn run_model_with_context(
		world: &AsyncWorld,
		agent: Entity,
		name: &str,
		instructions: &str,
	) -> Result<()> {
		// Create the action entity with ModelAction
		let action = world
			.spawn_then((
				Name::new(name.to_string()),
				ModelAction::new(OllamaProvider::default())
					.streaming()
					.with_instructions(instructions),
			))
			.await
			.id();

		// Build input items from context, treating this action as "self"
		let input_items: Vec<openresponses::request::InputItem> = world
			.with_then(move |world| {
				let mut items = Vec::new();

				// Query context entities associated with this agent
				let mut query = world.query::<(
					&ThreadContextOf,
					Option<&TextContext>,
					Option<&OwnedContextOf>,
				)>();

				for (context_of, text, meta) in query.iter(world) {
					if **context_of != agent {
						continue;
					}

					// Determine role based on who created this context
					let owner = meta.map(|m| m.get());
					let effective_role = if owner == Some(action) {
						openresponses::MessageRole::Assistant
					} else {
						openresponses::MessageRole::User
					};

					if let Some(text) = text {
						// Prefix with creator's name if not self
						let text_content = if owner != Some(action) {
							if let Some(owner_entity) = owner {
								if let Some(owner_name) =
									world.get::<Name>(owner_entity)
								{
									format!("{} > {}", owner_name, text.0)
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

		// Send to model using the standard approach
		let provider = OllamaProvider::default();
		let body =
			openresponses::RequestBody::new(provider.default_small_model())
				.with_input_items(input_items)
				.with_instructions(instructions)
				.with_stream(true);

		let mut stream = provider.stream(body).await?;
		let mut spawner =
			StreamingContextSpawner::new(world.clone(), agent, action);

		while let Some(event) = stream.next().await {
			let event = event?;
			match spawner.handle_event(&event).await? {
				std::ops::ControlFlow::Continue(_) => {}
				std::ops::ControlFlow::Break(_) => break,
			}
		}

		Ok(())
	}
}
