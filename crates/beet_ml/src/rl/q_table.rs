use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
pub type QValue = f32;

#[derive(Debug, Clone, PartialEq, Component, Deref, DerefMut, Reflect)]
#[reflect(Default)]
pub struct QTable<State: StateSpace, Action: ActionSpace>(
	pub HashMap<State, HashMap<Action, QValue>>,
);

impl<State: StateSpace, Action: ActionSpace> Default for QTable<State, Action> {
	fn default() -> Self { Self(HashMap::default()) }
}

impl<State: StateSpace, Action: ActionSpace> QSource for QTable<State, Action> {
	type Action = Action;
	type State = State;

	fn greedy_policy(&self, state: &Self::State) -> (Self::Action, QValue) {
		let mut best_value = QValue::default();
		let mut best_action = Self::Action::default();

		for (action, value) in self.get_actions(state) {
			if value > &best_value {
				best_value = *value;
				best_action = action.clone();
			}
		}

		(best_action, best_value)
	}

	fn get_actions(
		&self,
		state: &Self::State,
	) -> impl Iterator<Item = (&Self::Action, &QValue)> {
		self.get(state)
			.into_iter()
			.flat_map(|actions| actions.iter())
	}

	fn get_q(&self, state: &Self::State, action: &Self::Action) -> QValue {
		self.get(state)
			.and_then(|actions| actions.get(action))
			.copied()
			.unwrap_or_default()
	}

	fn set_q(
		&mut self,
		state: &Self::State,
		action: &Self::Action,
		value: QValue,
	) {
		self.entry(state.clone())
			.or_insert_with(HashMap::default)
			.insert(action.clone(), value);
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use rand::rngs::StdRng;
	use rand::SeedableRng;
	use sweet::*;


	/// This test is *almost* identical to a [`QTableTrainer`] but demonstrates
	/// that we can do realtime stuff and dont need to use an [`Environment`]
	#[test]
	fn works() -> Result<()> {
		let mut source = QTable::<GridPos, GridDirection>::default();
		let env = FrozenLakeEnv::default();
		let initial_state = env.state();
		let params = QLearnParams::default();
		let mut rng = StdRng::seed_from_u64(0);

		for episode in 0..params.n_training_episodes {
			let mut prev_state = initial_state.clone();
			let epsilon = params.epsilon(episode);

			let mut env = env.clone();
			let mut action = source
				.epsilon_greedy_policy(&prev_state, epsilon, &mut rng)
				.0;

			for _step in 0..params.max_steps {
				let outcome = env.step(&action);
				// Must step even if outcome is done, to remember reward
				action = source.step(
					&params,
					&mut rng,
					epsilon,
					&action,
					&prev_state,
					&outcome.state,
					outcome.reward,
				);
				if outcome.done {
					break;
				}
				prev_state = outcome.state;
			}
		}

		let eval =
			QTableTrainer::new(FrozenLakeEnv::default(), source).evaluate();

		expect(eval.mean).to_be(1.)?;
		expect(eval.std).to_be(0.)?;
		expect(eval.total_steps).to_be(600)?;


		Ok(())
	}
}
