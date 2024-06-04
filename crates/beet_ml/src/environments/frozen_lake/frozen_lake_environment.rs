use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;

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
	pub pos: UVec2,
	width: usize,
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
			width: grid.width(),
			is_slippery,
			pos: grid.agent_position().expect("No agent position found"),
			outcomes: grid.transition_outcomes(),
		}
	}

	pub fn pos_index(&self) -> usize {
		self.pos.x as usize + self.pos.y as usize * self.width
	}
}

impl Environment for FrozenLakeEnv {
	type State = usize;
	type Action = TranslateGridDirection;

	fn state(&self) -> Self::State { self.pos_index() }

	fn step(
		&mut self,
		action: impl Into<Self::Action>,
	) -> StepOutcome<Self::State> {
		let action = if self.is_slippery {
			action.into().as_slippery()
		} else {
			action.into()
		};
		let TransitionOutcome {
			pos,
			reward,
			is_terminal,
		} = self.outcomes[&(self.pos, action)];
		// println!("pos: {:?}, reward: {:?}, is_terminal: {:?}", pos, reward, is_terminal);

		self.pos = pos;
		StepOutcome {
			state: self.pos_index(),
			reward,
			done: is_terminal,
		}
	}
}
