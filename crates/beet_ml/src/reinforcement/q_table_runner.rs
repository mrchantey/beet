use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component)]
pub struct QTableRunner {
	params: QLearnParams,
	step: u32,
	episode: u32,
	epsilon: f32,
}

impl Default for QTableRunner {
	fn default() -> Self { Self::new(QLearnParams::default()) }
}

impl QTableRunner {
	pub fn new(params: QLearnParams) -> Self {
		Self {
			step: 0,
			episode: 0,
			epsilon: params.max_epsilon,
			params,
		}
	}

	pub fn should_run_episode(&self) -> bool {
		self.episode < self.params.n_training_episodes
	}


	pub fn next_episode(&mut self) {
		self.episode += 1;
		self.step = 0;
		self.epsilon = self.params.min_epsilon
			+ (self.params.max_epsilon - self.params.min_epsilon)
				* (-self.params.decay_rate * self.episode as f32).exp();
	}

	pub fn should_run_step(&self) -> bool { self.step < self.params.max_steps }
	pub fn epsilon(&self) -> f32 { self.epsilon }
}
