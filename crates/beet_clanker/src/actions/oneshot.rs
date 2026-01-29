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
pub fn simple_oneshot() -> impl Bundle {
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
pub fn simple_oneshot_streaming() -> impl Bundle {
	(Sequence, children![
		request_to_context(),
		ModelAction::new(OllamaProvider::default()).streaming(),
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
			.spawn(flow_exchange(simple_oneshot))
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
			.spawn(flow_exchange(simple_oneshot_streaming))
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
