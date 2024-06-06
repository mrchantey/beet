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
		Table: QSource<State = S, Action = A>,
	> QTableTrainer<S, A, Env, Table>
{
	pub fn new(env: Env, table: Table) -> Self {
		Self {
			table,
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
	> QSource for QTableTrainer<S, A, Env, Table>
{
	type Action = A;
	type State = S;
	fn greedy_policy(&self, state: &Self::State) -> (Self::Action, QValue) {
		self.table.greedy_policy(state)
	}

	fn get_actions(
		&self,
		state: &Self::State,
	) -> impl Iterator<Item = (&Self::Action, &QValue)> {
		self.table.get_actions(state)
	}

	fn get_q(&self, state: &Self::State, action: &Self::Action) -> QValue {
		self.table.get_q(state, action)
	}

	fn set_q(
		&mut self,
		state: &Self::State,
		action: &Self::Action,
		value: QValue,
	) {
		self.table.set_q(state, action, value)
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
			let epsilon = params.next_epsilon(episode);
			let mut env = self.env.clone();
			let mut state = env.state();

			'step: for _step in 0..params.max_steps {
				let (action, _) =
					self.table.epsilon_greedy_policy(&state, epsilon, rng);
				let outcome = env.step(&action);

				self.table.set_discounted_reward(
					params,
					&action,
					outcome.reward,
					&state,
					&outcome.state,
				);
				state = outcome.state;

				if outcome.done {
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
			let mut state = env.state();
			let mut total_reward = 0.0;

			for _step in 0..self.params.max_steps {
				total_steps += 1;
				let (action, _) = self.table.greedy_policy(&state);
				let outcome = env.step(&action);
				total_reward += outcome.reward;
				state = outcome.state;

				if outcome.done {
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
		let env =
			FrozenLakeEnv::new(map, false).with_slippery_rng(slippery_rng);

		let mut trainer = QTableTrainer::new(env.clone(), QTable::default());
		let now = Instant::now();
		trainer.train(&mut policy_rng);
		let elapsed = now.elapsed();
		println!("\nTrained in: {:.3?} seconds\n", elapsed.as_secs_f32());
		// println!("trained table: {:?}", table);
		expect(elapsed).to_be_greater_than(Duration::from_millis(2))?;
		// should be about 10ms
		expect(elapsed).to_be_less_than(Duration::from_millis(20))?;

		let evaluation = trainer.evaluate();
		println!("{evaluation:?}\n");

		expect(evaluation.mean).to_be_greater_than(0.99)?;
		expect(evaluation.std).to_be_close_to(0.00)?;
		expect(evaluation.total_steps).to_be(600)?;

		Ok(())
	}
}
