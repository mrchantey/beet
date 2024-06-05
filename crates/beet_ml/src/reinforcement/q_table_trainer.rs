use super::*;
use rand::Rng;


pub struct QTableTrainer<
	S: StateSpace,
	A: ActionSpace,
	Env: Environment<State = S, Action = A>,
	Table: QSource<State = S, Action = A>,
> {
	pub table: Table,
	pub env: Readonly<Env>,
	pub params: Readonly<QLearnParams>,
}

impl<
		S: StateSpace,
		A: ActionSpace,
		Env: Environment<State = S, Action = A>,
		// Table: QSource<State = S, Action = A> + Default,
	> QTableTrainer<S, A, Env, QTable<S, A>>
{
	pub fn new(env: Env) -> Self {
		Self {
			table: QTable::default(),
			env: Readonly::new(env),
			params: Readonly::new(QLearnParams::new()),
		}
	}
}


impl<
		S: StateSpace,
		A: ActionSpace,
		Env: Environment<State = S, Action = A>,
		Table: QSource<State = S, Action = A>,
	> QTrainer for QTableTrainer<S, A, Env, Table>
{
	fn train(&mut self, rng: &mut impl Rng) {
		let params = &self.params;

		for episode in 0..params.n_training_episodes {
			let epsilon = params.min_epsilon
				+ (params.max_epsilon - params.min_epsilon)
					* (-params.decay_rate * episode as f32).exp();

			let mut env = self.env.clone();
			let mut prev_state = env.state();

			'step: for _step in 0..params.max_steps {
				let (action, _) =
					self.table.epsilon_greedy_policy(&prev_state, epsilon, rng);
				let StepOutcome {
					state: new_state,
					reward: new_reward,
					done,
				} = env.step(&action);

				let prev_q = self.table.get_q(&prev_state, &action);
				let (_, new_max_q) = self.table.greedy_policy(&new_state);

				// Update using Bellman equation
				// Q(s,a):= Q(s,a) + lr [R(s,a) + gamma * max Q(s',a') - Q(s,a)]
				let discounted_reward = prev_q
					+ params.learning_rate
						* (new_reward + params.gamma * new_max_q - prev_q);

				self.table.set_q(&prev_state, &action, discounted_reward);
				prev_state = new_state;

				if done {
					break 'step;
				}
			}
		}
	}
	///   Evaluate using greedy policy for [`Self::n_eval_episodes`] episodes.
	fn evaluate(&self) -> Evaluation {
		let mut rewards = Vec::new();
		let mut total_steps = 0;
		let params = &self.params;
		for _episode in 0..params.n_eval_episodes {
			let mut env = self.env.clone();
			let mut prev_state = env.state();
			let mut total_reward = 0.0;

			for _step in 0..self.params.max_steps {
				total_steps += 1;
				let (action, _) = self.table.greedy_policy(&prev_state);
				let StepOutcome {
					state,
					reward,
					done,
				} = env.step(&action);
				total_reward += reward;
				prev_state = state;

				if done {
					break;
				}
			}
			rewards.push(total_reward);
		}
		Evaluation::new(rewards, total_steps)
	}
}





#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use rand::rngs::StdRng;
	use rand::SeedableRng;
	use std::time::Duration;
	use std::time::Instant;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let slippery_rng = StdRng::seed_from_u64(0);
		let mut policy_rng = StdRng::seed_from_u64(10);

		let map = FrozenLakeMap::default_four_by_four();
		let env = FrozenLakeEnv::new(map, false)
			.with_slippery_rng(slippery_rng.clone());


		let mut trainer = QTableTrainer::new(env.clone());
		let now = Instant::now();
		trainer.train(&mut policy_rng);
		let elapsed = now.elapsed();
		println!("\nTrained in: {:.2?} seconds\n", elapsed.as_secs_f32());
		// println!("trained table: {:?}", table);
		expect(elapsed).to_be_greater_than(Duration::from_millis(2))?;
		expect(elapsed).to_be_less_than(Duration::from_millis(100))?;

		let evaluation = trainer.evaluate();
		println!("{evaluation:?}\n");

		expect(evaluation.mean).to_be_greater_than(0.99)?;
		expect(evaluation.std).to_be_close_to(0.00)?;
		expect(evaluation.total_steps).to_be(600)?;

		Ok(())
	}
}
