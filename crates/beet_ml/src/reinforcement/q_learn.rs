use crate::prelude::*;
use rand::Rng;


pub trait QTrainer: 'static + Send + Sync {
	fn train(&mut self, rng: &mut impl Rng);
	fn evaluate(&self) -> Evaluation;
}


