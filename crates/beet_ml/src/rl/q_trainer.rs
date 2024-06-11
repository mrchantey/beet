use crate::prelude::*;
use rand::Rng;


pub trait QTrainer: 'static + Send + Sync + QPolicy {
	fn train(&mut self) { self.train_with_rng(&mut rand::thread_rng()) }
	/// Immediately train an entire agent
	fn train_with_rng(&mut self, rng: &mut impl Rng);
	/// Immediately evaluate an entire agent
	fn evaluate(&self) -> Evaluation;

	// fn reward(&mut self, reward: Reward);
}
