use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
type QValue = f32;
use rand::Rng;


pub trait QSource: 'static + Send + Sync {
	type State: StateSpace;
	type Action: ActionSpace;
	fn greedy_policy(&self, state: &Self::State) -> (Self::Action, QValue);
	fn epsilon_greedy_policy(
		&self,
		state: &Self::State,
		epsilon: f32,
		rng: &mut impl Rng,
	) -> (Self::Action, QValue) {
		let random_num: f32 = rng.gen(); // generates a float between 0 and 1
		if random_num > epsilon {
			// Exploitation: Take the action with the highest value given a state
			self.greedy_policy(state)
		} else {
			// Exploration: Take a random action
			(Self::Action::sample(rng), QValue::default())
		}
	}

	fn get_actions(
		&self,
		state: &Self::State,
	) -> impl Iterator<Item = (&Self::Action, &QValue)>;

	fn get_q(&self, state: &Self::State, action: &Self::Action) -> QValue;

	fn set_q(
		&mut self,
		state: &Self::State,
		action: &Self::Action,
		value: QValue,
	);
}

#[derive(Debug, Clone, PartialEq, Component, Deref, DerefMut, Reflect)]
#[reflect(Default)]
pub struct QTable<State: StateSpace, Action: ActionSpace>(
	pub HashMap<State, HashMap<Action, QValue>>,
);

impl<State: StateSpace, Action: ActionSpace> Default for QTable<State, Action> {
	fn default() -> Self { Self(HashMap::default()) }
}

// impl<const NUM_STATES: usize, const NUM_ACTIONS: usize> Default
// 	for QTable<{ NUM_STATES }, { NUM_ACTIONS }>
// {
// 	fn default() -> Self { Self([[0.0; NUM_ACTIONS]; NUM_STATES]) }
// }

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
		// println!(
		// 	"Setting Q value for state {:?} and action {:?} to {}",
		// 	state, action, value
		// );
		self.entry(state.clone())
			.or_insert_with(HashMap::default)
			.insert(action.clone(), value);
	}
}


// // impl<const NUM_STATES: usize, const NUM_ACTIONS: usize> Default
// // 	for QTable<{ NUM_STATES }, { NUM_ACTIONS }>
// // {
// // 	fn default() -> Self { Self([[0.0; NUM_ACTIONS]; NUM_STATES]) }
// // }
// /// A memoized QTable that stores the best action for each state.
// /// Every time a value is set, the best action for that state is updated.
// pub struct MemoizedQTable<const NUM_STATES: usize, const NUM_ACTIONS: usize> {
// 	table: QTable<{ NUM_STATES }, { NUM_ACTIONS }>,
// 	best_index: [usize; NUM_STATES],
// }

// impl<const NUM_STATES: usize, const NUM_ACTIONS: usize>
// 	MemoizedQTable<{ NUM_STATES }, { NUM_ACTIONS }>
// {
// 	pub fn new() -> Self {
// 		Self {
// 			table: QTable::default(),
// 			best_index: [0; NUM_STATES],
// 		}
// 	}
// 	pub fn table(&self) -> &QTable<{ NUM_STATES }, { NUM_ACTIONS }> {
// 		&self.table
// 	}
// 	pub fn winners(&self) -> &[usize; NUM_STATES] { &self.best_index }
// }

// impl<const NUM_STATES: usize, const NUM_ACTIONS: usize> QSource
// 	for MemoizedQTable<{ NUM_STATES }, { NUM_ACTIONS }>
// {
// 	const NUM_ACTIONS: usize = NUM_ACTIONS;
// 	fn greedy_policy(&self, state: Self::State) -> usize {
// 		self.best_index[state]
// 	}

// 	fn get_q(&self, state: Self::State, action: Self::Action) -> QValue {
// 		self.table.get_q(state, action)
// 	}
// 	fn set_q(
// 		&mut self,
// 		state: Self::State,
// 		action: Self::Action,
// 		value: QValue,
// 	) {
// 		self.table.set_q(state, action, value);
// 		let mut max_value: QValue = 0.;
// 		let mut max_index = 0;

// 		for (index, &value) in self.table[state].iter().enumerate() {
// 			if value > max_value {
// 				max_value = value;
// 				max_index = index;
// 			}
// 		}
// 		self.best_index[state] = max_index;
// 	}

// 	fn get_actions(&self, state: usize) -> impl Iterator<Item = &QValue> {
// 		self.table.get_actions(state)
// 	}
// }
