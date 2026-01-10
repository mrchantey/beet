use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;



impl ExchangeSpawner {
	/// Create a new ExchangeSpawner compatible with control flow structures.
	///
	/// This spawns two separate entities:
	/// 1. **Agent entity**: Holds the `Request` and `Response` components
	/// 2. **Action root entity**: Child of agent with `ActionOf(agent)`, contains the behavior tree
	///
	/// ## Execution Flow
	///
	/// 1. [`Request`] is inserted on the agent entity
	/// 2. [`GetOutcome`] is triggered on the action root entity
	/// 3. The behavior tree executes (may insert [`Response`] at any point)
	/// 4. An [`Outcome`] is triggered on the action root:
	///    - If no [`Response`] exists, a default is inserted based on the outcome:
	///      - [`Outcome::Pass`] → [`StatusCode::OK`]
	///      - [`Outcome::Fail`] → [`StatusCode::INTERNAL_SERVER_ERROR`]
	///    - [`ExchangeComplete`] event is triggered on the agent to signal completion
	/// 5. `handle_request` observes [`ExchangeComplete`] event, takes the [`Response`], and returns it
	///
	/// ## Important
	///
	/// Actions in the behavior tree **must** trigger an [`Outcome`] to complete the exchange.
	/// Without an [`Outcome`], the exchange will hang indefinitely waiting for [`ExchangeComplete`].
	pub fn new_flow(func: impl BundleFunc) -> Self {
		Self::new(move |world: &mut World| {
			let func = func.clone();

			// Spawn the agent entity first (without the action root yet)
			let agent = world.spawn(Name::new("Flow Exchange Agent")).id();

			// Now spawn the action root as a separate entity with ActionOf pointing to agent
			let action_root = world
				.spawn((
					Name::new("Flow Exchange Action"),
					ActionOf(agent),
					// when Outcome is triggered on this entity, ensure response exists and trigger complete
					OnSpawn::observe(
						|ev: On<Outcome>,
						 agents: AgentQuery,
						 mut commands: Commands,
						 has_response: Query<(), With<ResponseMarker>>| {
							let action = ev.target();
							let agent = agents.entity(action);
							// Insert default response if none exists
							if !has_response.contains(agent) {
								let status = match ev.event() {
									Outcome::Pass => StatusCode::OK,
									Outcome::Fail => {
										StatusCode::INTERNAL_SERVER_ERROR
									}
								};
								commands
									.entity(agent)
									.insert(Response::from_status(status));
							}
							// Signal completion to handle_request
							commands
								.entity(agent)
								.trigger_target(ExchangeComplete);
						},
					),
					func.bundle_func(),
				))
				.id();

			// Add the Request observer to the agent
			world.entity_mut(agent).insert(OnSpawn::observe(
				move |_ev: On<Insert, Request>, mut commands: Commands| {
					// When Request is inserted on agent, trigger GetOutcome on the action root
					commands.entity(action_root).trigger_target(GetOutcome);
				},
			));
			// Return the agent entity (where Request and Response live)
			agent
		})
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;


	#[sweet::test]
	async fn flow_inserts_response() {
		use beet_flow::prelude::*;
		ServerPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| {
				OnSpawn::observe(
					|ev: On<GetOutcome>,
					 agents: AgentQuery,
					 mut commands: Commands| {
						let action = ev.target();
						let agent = agents.entity(action);
						commands.entity(agent).insert(Response::from_status(
							StatusCode::IM_A_TEAPOT,
						));
						commands.entity(action).trigger_target(Outcome::Pass);
					},
				)
			}))
			.oneshot(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}

	#[sweet::test]
	async fn flow_outcome_pass() {
		use beet_flow::prelude::*;
		ServerPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| EndWith(Outcome::Pass)))
			.oneshot(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[sweet::test]
	async fn flow_outcome_fail() {
		use beet_flow::prelude::*;
		ServerPlugin::world()
			.spawn(ExchangeSpawner::new_flow(|| EndWith(Outcome::Fail)))
			.oneshot(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
	}

	#[sweet::test]
	async fn agent_is_separate_from_action_root() {
		use beet_flow::prelude::*;

		// Verify that the agent and action root are separate entities
		let mut world = ServerPlugin::world();
		world.spawn(ExchangeSpawner::new_flow(|| {
			OnSpawn::observe(
				|ev: On<GetOutcome>,
				 agents: AgentQuery,
				 mut commands: Commands| {
					let action = ev.target();
					let agent = agents.entity(action);
					agent.xpect_not_eq(action);
					agent.xpect_not_eq(agents.parents.root_ancestor(action));
					commands
						.entity(agent)
						.insert(Response::from_status(StatusCode::IM_A_TEAPOT));
					commands.entity(action).trigger_target(Outcome::Pass);
				},
			)
		}));

		world
			.oneshot(Request::get("foo"))
			.await
			.status()
			.xpect_eq(StatusCode::IM_A_TEAPOT);
	}
}
