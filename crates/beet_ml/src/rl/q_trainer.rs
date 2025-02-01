use crate::prelude::*;
use sweet::prelude::Rng;

pub trait QTrainer: 'static + Send + Sync + QPolicy {
	/// Immediately train an entire agent
	fn train(&mut self, rng: &mut impl Rng);
	/// Immediately evaluate an entire agent
	fn evaluate(&self) -> Evaluation;

	// fn reward(&mut self, reward: Reward);
}
