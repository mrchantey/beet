use crate::prelude::*;
use beet_core::prelude::*;


/// Used for training a QTable to completion with a provided [`Environment`].
pub struct QTableTrainer<S: RlSessionTypes> {
	pub table: S::QLearnPolicy,
	pub env: Readonly<S::Env>,
	pub params: Readonly<QLearnParams>,
	initial_state: S::State,
}

impl<S: RlSessionTypes> QTableTrainer<S> {
	pub fn new(
		env: S::Env,
		table: S::QLearnPolicy,
		params: QLearnParams,
		initial_state: S::State,
	) -> Self {
		Self {
			table,
			env: Readonly::new(env),
			params: Readonly::new(params),
			initial_state,
		}
	}
}


impl<S: RlSessionTypes> QPolicy for QTableTrainer<S> {
	type Action = S::Action;
	type State = S::State;
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


impl<S: RlSessionTypes> QTrainer for QTableTrainer<S>
where
	S::State: Clone,
{
	fn train(&mut self, rng: &mut impl Rng) {
		let params = &self.params;

		for episode in 0..params.n_training_episodes {
			let epsilon = params.epsilon(episode);
			let mut env = self.env.clone();
			let mut state = self.initial_state.clone();

			'step: for _step in 0..params.max_steps {
				// 1. select action
				let (action, _) =
					self.table.epsilon_greedy_policy(&state, epsilon, rng);
				// 2. step environent
				let outcome = env.step(&state, &action);
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
			let mut state = self.initial_state.clone();
			let mut total_reward = 0.0;

			for _step in 0..self.params.max_steps {
				total_steps += 1;
				let (action, _) = self.table.greedy_policy(&state);
				let outcome = env.step(&state, &action);
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
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut policy_rng = RandomSource::from_seed(0);
		let map = FrozenLakeMap::default_four_by_four();
		let initial_state = map.agent_position();
		let env = QTableEnv::new(map.transition_outcomes());
		let params = QLearnParams::default();

		let mut trainer = QTableTrainer::<FrozenLakeQTableSession>::new(
			env.clone(),
			QTable::default(),
			params,
			initial_state,
		);
		let now = Instant::now();
		trainer.train(&mut policy_rng.0);
		// My PC: 10ms
		// Github Actions: 50ms
		let elapsed = now.elapsed();
		// println!("\nTrained in: {:.3?} seconds\n", elapsed.as_secs_f32());
		// println!("trained table: {:?}", table);
		elapsed.xpect_greater_than(Duration::from_millis(2));

		let eval = trainer.evaluate();
		// println!("{eval:?}\n");

		// optimal policy = 6 steps & 100
		eval.mean.xpect_eq(1.);
		eval.std.xpect_eq(0.);
		eval.total_steps.xpect_eq(600);
	}
}
