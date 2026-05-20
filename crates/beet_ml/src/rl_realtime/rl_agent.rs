use crate::prelude::*;
use beet_core::prelude::*;

/// Bundle spawned for each agent inside an RL episode. Carries the
/// agent's discrete state and action, its environment, the shared
/// hyper-parameters, and lifecycle markers.
#[derive(Bundle)]
pub struct RlAgentBundle<Env: Component + Environment> {
	/// Current discrete state.
	pub state: Env::State,
	/// Last chosen discrete action.
	pub action: Env::Action,
	/// Environment driving transitions.
	pub env: Env,
	/// Shared hyper-parameters.
	pub params: QLearnParams,
	/// Session that owns this agent.
	pub session: SessionEntity,
	/// Despawned when the episode ends.
	pub despawn: DespawnOnEpisodeEnd,
}
