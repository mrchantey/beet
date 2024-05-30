use crate::prelude::*;
use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;
use strum::VariantArray;


fn categorical_sample(
	outcomes: &[TransitionOutcome],
	random_val: f32,
) -> &TransitionOutcome {
	let mut sum = 0.;
	let summed = outcomes
		.iter()
		.map(|o| {
			sum += o.prob;
			sum
		})
		.collect::<Vec<_>>();
	assert!(sum <= 1.);
	let index = summed
		.iter()
		.position(|&x| x > random_val)
		.unwrap_or_default();
	&outcomes[index]
}


/// Directions for the agent to move in.

/// The outcome of a transition.
pub struct TransitionOutcome {
	/// The probability of this outcome.
	pub prob: f32,
	/// The new position of the agent.
	pub pos: UVec2,
	/// The reward obtained from the transition.
	pub reward: f32,
	/// Whether the new state is terminal.
	pub is_terminal: bool,
}

/// The environment for the Frozen Lake game.
pub struct FrozenLakeEnv {
	/// The grid of cells.
	// pub grid: Vec<Vec<Cell>>,
	/// The current position of the agent.
	pub position: UVec2,
	/// Whether the environment is slippery, meaning the agent doesn't always move in the intended direction.
	pub is_slippery: bool,
	/// The transition probabilities for each state-action pair.
	pub transition_probs:
		HashMap<(UVec2, TranslateGridDirection), Vec<TransitionOutcome>>,
}

impl FrozenLakeEnv {
	/// Creates a new environment.
	/// # Panics
	/// If the map has no agent position.
	pub fn new<const L: usize>(
		grid: FrozenLakeMap<L>,
		is_slippery: bool,
	) -> Self {
		let mut transition_probs = HashMap::new();

		for (index, cell) in grid.tiles().iter().enumerate() {
			let pos = index_to_position(index, grid.width());
			// let i = i as u32;
			// let j = j as u32;

			for &action in &[
				TranslateGridDirection::Left,
				TranslateGridDirection::Down,
				TranslateGridDirection::Right,
				TranslateGridDirection::Up,
			] {
				let mut outcomes = Vec::<TransitionOutcome>::new();

				if cell.is_terminal() {
					outcomes.push(TransitionOutcome {
						prob: 1.0,
						reward: 0.0,
						pos,
						is_terminal: true,
					});
				} else {
					let directions = if is_slippery {
						vec![
							(action as i32 - 1).rem_euclid(4) as usize,
							action as usize,
							(action as i32 + 1).rem_euclid(4) as usize,
						]
					} else {
						vec![action as usize]
					};

					for &direction in &directions {
						let new_pos =
							match TranslateGridDirection::VARIANTS[direction] {
								TranslateGridDirection::Left => {
									UVec2::new(pos.x, pos.y.saturating_sub(1))
								}
								TranslateGridDirection::Down => {
									UVec2::new(pos.x.saturating_add(1), pos.y)
								}
								TranslateGridDirection::Right => {
									UVec2::new(pos.x, pos.y.saturating_add(1))
								}
								TranslateGridDirection::Up => {
									UVec2::new(pos.x.saturating_sub(1), pos.y)
								}
							};

						// println!("new pos: {:?}", new_pos);

						let new_index =
							position_to_index(new_pos, grid.width());
						if new_index >= grid.tiles().len() {
							continue;
						}


						let new_cell = grid.tiles()[new_index];
						let reward = if new_cell == FrozenLakeTile::Goal {
							1.0
						} else {
							0.0
						};

						outcomes.push(TransitionOutcome {
							prob: 1.0 / directions.len() as f32,
							pos: new_pos,
							reward,
							is_terminal: new_cell.is_terminal(),
						});
					}

					transition_probs.insert((pos, action), outcomes);
				}
			}
		}

		Self {
			position: grid.agent_position().expect("map has no agent position"),
			is_slippery,
			transition_probs,
		}
	}

	/// Takes a step in the environment.
	pub fn step(&mut self, action: TranslateGridDirection) -> StepResult {
		let outcomes = &self.transition_probs[&(self.position, action)];
		let mut rng = rand::thread_rng();
		let outcome = categorical_sample(outcomes, rng.gen::<f32>());
		StepResult {
			new_pos: self.position,
			reward: outcome.reward,
			is_terminal: outcome.is_terminal,
		}

		// (self.position.0, self.position.1, outcome.2, outcome.3)
	}
}

#[derive(Debug, Clone)]
pub struct StepResult {
	/// The new row position of the agent.
	pub new_pos: UVec2,
	/// The reward obtained from the step.
	pub reward: f32,
	/// Whether the new state is terminal.
	pub is_terminal: bool,
}
