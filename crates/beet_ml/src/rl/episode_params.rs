use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;

/// Configuration carried by an RL session, parameterising how many
/// episodes to run.
pub trait EpisodeParams: 'static + Send + Sync + std::fmt::Debug + Clone + Reflect {
	/// Number of training episodes in the session.
	fn num_episodes(&self) -> u32;
}

/// Type-level bundle of the (State, Action, Policy, Env, EpisodeParams)
/// associated with a single RL session. Concrete implementations like
/// `FrozenLakeQTableSession` thread this tuple through trainers and
/// session-level systems.
pub trait RlSessionTypes: 'static + Send + Sync + Reflect {
	/// Discrete observation space, stored on the agent entity.
	type State: StateSpace + Component<Mutability = Mutable>;
	/// Discrete action space, stored on the agent entity.
	type Action: ActionSpace + Component<Mutability = Mutable>;
	/// Policy mutated during training; stored on the session entity.
	type QLearnPolicy: Component<Mutability = Mutable>
		+ QPolicy<State = Self::State, Action = Self::Action>;
	/// Environment driving state transitions; stored on the agent.
	type Env: Environment<State = Self::State, Action = Self::Action>
		+ Component<Mutability = Mutable>;
	/// Session-level configuration carried through messages.
	type EpisodeParams: EpisodeParams;
}
