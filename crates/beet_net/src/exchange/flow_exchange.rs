//! Exchange pattern for behavior tree control flow integration.
//!
//! This module provides [`flow_exchange`], which creates an exchange handler
//! compatible with [`beet_flow`] control flow structures like [`Sequence`], [`Fallback`],
//! and other behavior tree patterns.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

/// Creates an exchange handler compatible with control flow structures.
///
/// This spawns two separate entities for each exchange:
/// 1. **Agent entity**: Holds the [`Request`], [`Response`], and [`ExchangeEnd`] components
/// 2. **Action root entity**: Child of agent with [`ActionOf`] pointing to agent, contains the behavior tree
///
/// ## Execution Flow
///
/// 1. [`ExchangeStart`] is triggered on the spawner entity
/// 2. Agent and action root entities are spawned
/// 3. [`GetOutcome`] is triggered on the action root entity
/// 4. The behavior tree executes (may insert [`Response`] at any point)
/// 5. An [`Outcome`] is triggered on the action root:
///    - If no [`Response`] exists, a default is inserted based on the outcome:
///      - [`Outcome::Pass`] → [`StatusCode::Ok`]
///      - [`Outcome::Fail`] → [`StatusCode::InternalError`]
///    - The response is sent via [`ExchangeEnd`]
///
/// ## Important
///
/// Actions in the behavior tree **must** trigger an [`Outcome`] to complete the exchange.
/// Without an [`Outcome`], the exchange will hang indefinitely.
///
/// ## Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_flow::prelude::*;
/// let mut world = World::new();
/// let mut entity = world.spawn(flow_exchange(|| EndWith(Outcome::Pass)));
/// // Exchange will complete with StatusCode::Ok
/// ```
pub fn flow_exchange(func: impl BundleFunc) -> impl Bundle {
	OnSpawn::observe(
		move |ev: On<ExchangeStart>, mut commands: Commands| -> Result {
			let spawner_entity = ev.event_target();
			let ExchangeContext { request, end } = ev.take()?;

			// Spawn the agent entity with request and exchange end
			let agent = commands
				.spawn((
					Name::new("Flow Exchange Agent"),
					ChildOf(spawner_entity),
					request,
					end,
				))
				.id();

			// Spawn the action root with the behavior tree
			// Note: func() must come before OnSpawn::trigger so that any observers
			// registered by func() are ready before GetOutcome is triggered
			commands.spawn((
				Name::new("Flow Exchange Action"),
				ChildOf(agent),
				ActionOf(agent),
				// When Outcome is triggered, ensure response exists and send it
				OnSpawn::observe(outcome_handler),
				// User's behavior tree bundle (registers observers/actions)
				func.clone().bundle_func(),
				// Trigger GetOutcome after all observers are registered
				OnSpawn::trigger(GetOutcome),
			));

			Ok(())
		},
	)
}

/// Handles outcome events, inserting default response if needed and completing the exchange.
fn outcome_handler(
	ev: On<Outcome>,
	agents: AgentQuery,
	mut commands: Commands,
	has_response: Query<(), With<ResponseMarker>>,
) {
	let action = ev.target();
	let agent = agents.entity(action);

	// Insert default response if none exists
	if !has_response.contains(agent) {
		let status = match ev.event() {
			Outcome::Pass => StatusCode::Ok,
			Outcome::Fail => StatusCode::InternalError,
		};
		commands.entity(agent).insert(Response::from_status(status));
	}

	// Send the response
	commands.entity(agent).queue(take_and_send_response);
}

fn take_and_send_response(mut entity: EntityWorldMut) -> Result {
	let response = entity
		.take::<Response>()
		.unwrap_or_else(|| Response::not_found());
	entity
		.get::<ExchangeEnd>()
		.ok_or_else(|| bevyhow!("ExchangeEnd not found"))?
		.send(response)?;
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;

	#[beet_core::test]
	async fn outcome_pass() {
		World::new()
			.spawn(flow_exchange(|| EndWith(Outcome::Pass)))
			.exchange(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
	}

	#[beet_core::test]
	async fn outcome_fail() {
		World::new()
			.spawn(flow_exchange(|| EndWith(Outcome::Fail)))
			.exchange(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::InternalError);
	}

	#[beet_core::test]
	async fn custom_response() {
		World::new()
			.spawn(flow_exchange(|| {
				OnSpawn::observe(
					|ev: On<GetOutcome>,
						agents: AgentQuery,
						mut commands: Commands| {
						let action = ev.target();
						let agent = agents.entity(action);
						commands
							.entity(agent)
							.insert(Response::from_status(StatusCode::ImATeapot));
						commands.entity(action).trigger_target(Outcome::Pass);
					},
				)
			}))
			.exchange(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::ImATeapot);
	}

	#[beet_core::test]
	async fn agent_is_separate_from_action_root() {
		World::new()
			.spawn(flow_exchange(|| {
				OnSpawn::observe(
					|ev: On<GetOutcome>,
						agents: AgentQuery,
						mut commands: Commands| {
						let action = ev.target();
						let agent = agents.entity(action);
						// Verify agent and action are different entities
						agent.xpect_not_eq(action);
						commands.entity(action).trigger_target(Outcome::Pass);
					},
				)
			}))
			.exchange(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::Ok);
	}
}
