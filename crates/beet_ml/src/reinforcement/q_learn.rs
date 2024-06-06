use crate::prelude::*;
use rand::Rng;


pub trait QTrainer: 'static + Send + Sync {
	type State: StateSpace;
	type Action: ActionSpace;
	fn train(&mut self, rng: &mut impl Rng);
	fn evaluate(&self) -> Evaluation;
}
