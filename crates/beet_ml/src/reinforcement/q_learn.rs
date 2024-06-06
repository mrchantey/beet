use crate::prelude::*;
use rand::Rng;


pub trait QTrainer: 'static + Send + Sync + QSource {
	/// Immediately train an entire agent
	fn train(&mut self, rng: &mut impl Rng);
	/// Immediately evaluate an entire agent
	fn evaluate(&self) -> Evaluation;

	// fn reward(&mut self, reward: Reward);
}
