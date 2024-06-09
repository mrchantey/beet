use super::*;
use rand::Rng;



/// Used for training a QTable to completion with a provided [`Environment`].
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
				// 1. select action
				let (action, _) =
					self.table.epsilon_greedy_policy(&state, epsilon, rng);
				// 2. step environent
				let outcome = env.step(&action);
				// 3. update reward
				self.table.set_discounted_reward(
					params,
					&action,
					outcome.reward,
					&state,
					&outcome.state,
				);
				// 4. update state and break if done
				if outcome.done {
					break 'step;
				}
				state = outcome.state;
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
		let mut policy_rng = StdRng::seed_from_u64(0);

		let env = FrozenLakeEnv::default();

		let mut trainer = QTableTrainer::new(env.clone(), QTable::default());
		let now = Instant::now();
		trainer.train(&mut policy_rng);
		let elapsed = now.elapsed();
		// println!("\nTrained in: {:.3?} seconds\n", elapsed.as_secs_f32());
		// println!("trained table: {:?}", table);
		expect(elapsed).to_be_greater_than(Duration::from_millis(2))?;
		// should be about 10ms
		expect(elapsed).to_be_less_than(Duration::from_millis(30))?;

		let eval = trainer.evaluate();
		// println!("{eval:?}\n");

		// optimal policy
		expect(eval.mean).to_be(1.)?;
		expect(eval.std).to_be(0.)?;
		expect(eval.total_steps).to_be(600)?;

		Ok(())
	}
}
