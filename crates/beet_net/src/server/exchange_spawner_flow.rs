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
	/// Flow:
	/// 1. Upon a [`Request`] insert on the agent, [`GetOutcome`] will be triggered on the action root
	/// 2. Upon an [`Outcome`] on the action root, a response will be inserted on the agent if none exists:
	/// 	- [`Outcome::Pass`] -> [`StatusCode::OK`]
	/// 	- [`Outcome::Fail`] -> [`StatusCode::INTERNAL_SERVER_ERROR`]
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
					// when Outcome is triggered on this entity, insert default response on agent if needed
					OnSpawn::observe(
						|ev: On<Outcome>,
						 agents: AgentQuery,
						 mut commands: Commands,
						 has_response: Query<(), With<ResponseMarker>>| {
							let action = ev.target();
							let agent = agents.entity(action);
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
						},
					),
					func.bundle_func(),
				))
				.id();

			// Add the Request observer to the agent, which triggers GetOutcome on the action root
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
						let agent = agents.entity(ev.target());
						commands.entity(agent).insert(Response::from_status(
							StatusCode::IM_A_TEAPOT,
						));
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
