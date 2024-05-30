#![allow(dead_code)]
use super::QSource;
use super::QTable;
use crate::prelude::Environment;
use crate::prelude::Space;

// #[derive()]
pub struct FrozenLakeParameters {
	n_training_episodes: u32,
	learning_rate: f32,
	n_eval_episodes: u32,
	env_id: String,
	max_steps: u32,
	gamma: f32,
	eval_seed: Vec<u32>,
	max_epsilon: f32,
	min_epsilon: f32,
	decay_rate: f32,
}

impl Default for FrozenLakeParameters {
	fn default() -> Self { Self::new() }
}

impl FrozenLakeParameters {
	pub fn new() -> Self {
		Self {
			n_training_episodes: 10000,
			learning_rate: 0.7,
			n_eval_episodes: 100,
			env_id: "FrozenLake-v1".to_string(),
			max_steps: 99,
			gamma: 0.95,
			eval_seed: vec![],
			max_epsilon: 1.0,
			min_epsilon: 0.05,
			decay_rate: 0.0005,
		}
	}

	pub fn train<E: Environment, S: Space>(
		&mut self,
		table: &mut impl QSource,
	) {
		for episode in 0..self.n_training_episodes {
			let epsilon = self.min_epsilon
				+ (self.max_epsilon - self.min_epsilon)
					* (-self.decay_rate * episode as f32).exp();

			let state = E::default();


			for step in 0..self.max_steps {
				let action = table.epsilon_greedy_policy(0, epsilon);

				// new_state, reward, terminated, truncated, info = env.step(action)
			}
		}
	}
}

// pub fn train(
// 	n_training_episodes: usize,
// 	min_epsilon: f32,
// 	max_epsilon: f32,
// 	decay_rate: f32,
// 	max_steps: usize,
// 	q_table: &mut Array2<f32>,
// 	learning_rate: f32,
// 	gamma: f32,
// ) -> Array2<f32> {
// 	for episode in 0..n_training_episodes {}
// }
