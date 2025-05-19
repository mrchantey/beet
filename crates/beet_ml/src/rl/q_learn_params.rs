use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Reflect, Component)]
pub struct QLearnParams {
	pub n_training_episodes: u32,
	pub n_eval_episodes: u32,
	pub learning_rate: f32,
	pub max_steps: u32,
	pub gamma: f32,
	// pub eval_seed: u64,
	pub max_epsilon: f32,
	pub min_epsilon: f32,
	pub decay_rate: f32,
}

impl Default for QLearnParams {
	fn default() -> Self { Self::new() }
}

impl QLearnParams {
	pub fn new() -> Self {
		Self {
			// n_training_episodes: 1000,
			n_training_episodes: 10000,
			// n_training_episodes: 10,
			learning_rate: 0.7,
			n_eval_episodes: 100,
			max_steps: 99,
			gamma: 0.95,
			// eval_seed: u64,
			max_epsilon: 1.0,
			min_epsilon: 0.05,
			decay_rate: 0.0005,
		}
	}
	pub fn epsilon(&self, episode: u32) -> f32 {
		self.min_epsilon
			+ (self.max_epsilon - self.min_epsilon)
				* (-self.decay_rate * episode as f32).exp()
	}
}
