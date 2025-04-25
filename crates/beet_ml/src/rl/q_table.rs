use crate::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
pub type QValue = f32;

#[derive(
	Debug,
	Clone,
	PartialEq,
	Component,
	Deref,
	DerefMut,
	Reflect,
	Serialize,
	Deserialize,
	Asset,
)]
#[reflect(Default, Component)]
pub struct QTable<State: StateSpace, Action: ActionSpace>(
	pub HashMap<State, HashMap<Action, QValue>>,
);

impl<State: StateSpace, Action: ActionSpace> Default for QTable<State, Action> {
	fn default() -> Self { Self(HashMap::default()) }
}

impl<State: StateSpace, Action: ActionSpace> QPolicy for QTable<State, Action> {
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
	use sweet::prelude::*;

	/// This test is *almost* identical to a [`QTableTrainer`] but demonstrates
	/// that we can do realtime stuff and dont need to use an [`Environment`]
	#[test]
	fn works() {
		let mut source = QTable::<GridPos, GridDirection>::default();
		let params = QLearnParams::default();
		let mut rng = RandomSource::from_seed(0);
		let map = FrozenLakeMap::default_four_by_four();
		let initial_state = map.agent_position();
		let env = QTableEnv::new(map.transition_outcomes());

		for episode in 0..params.n_training_episodes {
			let mut state = initial_state.clone();
			let epsilon = params.epsilon(episode);

			let mut env = env.clone();
			let mut action =
				source.epsilon_greedy_policy(&state, epsilon, &mut rng.0).0;

			for _step in 0..params.max_steps {
				let outcome = env.step(&state, &action);
				// Must step even if outcome is done, to remember reward
				action = source.step(
					&params,
					&mut rng.0,
					epsilon,
					&action,
					&state,
					&outcome.state,
					outcome.reward,
				);
				if outcome.done {
					break;
				}
				state = outcome.state;
			}
		}

		let eval = QTableTrainer::<FrozenLakeQTableSession>::new(
			env,
			source,
			params,
			initial_state,
		)
		.evaluate();

		expect(eval.mean).to_be(1.);
		expect(eval.std).to_be(0.);
		expect(eval.total_steps).to_be(600);
	}
}
