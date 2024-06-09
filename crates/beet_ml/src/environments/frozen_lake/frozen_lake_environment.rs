use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// The outcome of a transition.
#[derive(Debug, Clone)]
pub struct TransitionOutcome {
	/// The new position of the agent.
	pub pos: UVec2,
	/// The reward obtained from the transition.
	pub reward: f32,
	/// Whether the new state is terminal.
	pub is_terminal: bool,
}

#[derive(Debug, Clone, Component)]
/// An environment for the Frozen Lake game.
pub struct FrozenLakeEnv {
	/// A number generator for determining.
	rng: StdRng,
	/// Whether there is a 2/3 chance the agent moves left or right of the intended direction.
	is_slippery: bool,
	/// The transition probabilities for each state-action pair.
	outcomes: HashMap<(UVec2, GridDirection), TransitionOutcome>,
}

impl FrozenLakeEnv {
	/// Creates a new environment.
	/// # Panics
	/// If the map has no agent position.
	pub fn new(grid: FrozenLakeMap, is_slippery: bool) -> Self {
		Self {
			is_slippery,
			rng: StdRng::from_entropy(),
			outcomes: grid.transition_outcomes(),
		}
	}
	pub fn with_slippery_rng(mut self, rng: StdRng) -> Self {
		self.rng = rng;
		self
	}
}

impl Environment for FrozenLakeEnv {
	type State = GridPos;
	type Action = GridDirection;


	fn step(
		&mut self,
		state: &Self::State,
		action: &Self::Action,
	) -> StepOutcome<Self::State> {
		let action = if self.is_slippery {
			action.as_slippery(&mut self.rng)
		} else {
			action.clone()
		};
		let TransitionOutcome {
			pos,
			reward,
			is_terminal,
		} = self.outcomes[&(**state, action)];
		// println!("pos: {:?}, reward: {:?}, is_terminal: {:?}", pos, reward, is_terminal);

		StepOutcome {
			state: pos.into(),
			reward,
			done: is_terminal,
		}
	}
}
