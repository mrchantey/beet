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
		 contexts: Query<(&ContextRole, &TextContext)>,
		 agents: AgentQuery<&ThreadContext>,
		 mut commands: Commands|
		 -> Result {
			let action = ev.target();
			let items = agents.get(action)?;
			let agent = agents.entity(action);

			let mut response_parts = Vec::new();
			for (role, text) in
				items.iter().filter_map(|entity| contexts.get(entity).ok())
			{
				if role == &ContextRole::Assistant {
					response_parts.push(text.0.clone());
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

	#[beet_core::test(timeout_ms = 30_000)]
	async fn model_to_model() {
		FlowAgentPlugin::world()
			.spawn(flow_exchange(|| {
				(Sequence, children![
					(Name::new("Darth Vader"), request_to_context()),
					(
						Name::new("Princess Leia"),
						ModelAction::new(OllamaProvider::default())
							.streaming()
							.with_instructions(
								"You are Princess Leia. Briefly introduce yourself"
							)
					),
					(
						Name::new("Luke Skywalker"),
						ModelAction::new(OllamaProvider::default())
							.streaming()
							.with_instructions(
								"You are Luke Skywalker. Briefly introduce yourself"
							)
					),
					context_to_response()
				])
			}))
			.exchange_str(
				Request::from_cli_str("I am Darth Vader, bow before me")
					.unwrap(),
			)
			.await
			// .xprint_display()
			.to_lowercase()
			.xpect_contains("princess leia")
			.xpect_contains("luke skywalker");
	}
}
