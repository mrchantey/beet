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

/// The environment for the Frozen Lake game.
pub struct FrozenLakeEnv {
	/// The position of the agent.
	pub state: GridPos,
	/// A number generator for determining.
	rng: StdRng,
	/// Whether there is a 2/3 chance the agent moves left or right of the intended direction.
	is_slippery: bool,
	/// The transition probabilities for each state-action pair.
	outcomes: HashMap<(UVec2, TranslateGridDirection), TransitionOutcome>,
}

impl FrozenLakeEnv {
	/// Creates a new environment.
	/// # Panics
	/// If the map has no agent position.
	pub fn new<const L: usize>(
		grid: FrozenLakeMap<L>,
		is_slippery: bool,
	) -> Self {
		Self {
			is_slippery,
			rng: StdRng::from_entropy(),
			state: grid
				.agent_position()
				.expect("No agent position found")
				.into(),
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
	type Action = TranslateGridDirection;

	fn state(&self) -> Self::State { self.state }

	fn step(&mut self, action: &Self::Action) -> StepOutcome<Self::State> {
		let action = if self.is_slippery {
			action.as_slippery(&mut self.rng)
		} else {
			action.clone()
		};
		let TransitionOutcome {
			pos,
			reward,
			is_terminal,
		} = self.outcomes[&(*self.state, action)];
		// println!("pos: {:?}, reward: {:?}, is_terminal: {:?}", pos, reward, is_terminal);

		self.state = pos.into();
		StepOutcome {
			state: self.state,
			reward,
			done: is_terminal,
		}
	}
}
