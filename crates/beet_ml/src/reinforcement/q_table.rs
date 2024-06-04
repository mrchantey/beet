use bevy::prelude::*;
type Value = f32;
use rand::Rng;


pub trait QSource {
	const NUM_ACTIONS: usize;
	fn greedy_policy(&self, state: usize) -> usize;
	fn epsilon_greedy_policy(&self, state: usize, epsilon: f32) -> usize {
		let mut rng = rand::thread_rng();
		let random_num: f32 = rng.gen(); // generates a float between 0 and 1
		if random_num > epsilon {
			// Exploitation: Take the action with the highest value given a state
			self.greedy_policy(state)
		} else {
			// Exploration: Take a random action
			rng.gen_range(0..Self::NUM_ACTIONS)
		}
	}

	fn get_actions(&self, state: usize) -> impl Iterator<Item = &Value>;

	fn get(&self, state: usize, action: usize) -> Value;

	fn set(&mut self, state: usize, action: usize, value: Value);
}

#[derive(Debug, Clone, PartialEq, Component, Deref, DerefMut)]
pub struct QTable<const NUM_STATES: usize, const NUM_ACTIONS: usize>(
	pub [[Value; NUM_ACTIONS]; NUM_STATES],
);

impl<const NUM_STATES: usize, const NUM_ACTIONS: usize> QSource
	for QTable<{ NUM_STATES }, { NUM_ACTIONS }>
{
	const NUM_ACTIONS: usize = NUM_ACTIONS;
	fn greedy_policy(&self, state: usize) -> usize {
		let mut max_value = self.0[state][0];
		let mut max_index = 0;

		for (index, &value) in self.0[state].iter().enumerate() {
			if value > max_value {
				max_value = value;
				max_index = index;
			}
		}

		max_index
	}

	fn get_actions(&self, state: usize) -> impl Iterator<Item = &Value> {
		self.0[state].iter()
	}

	fn get(&self, state: usize, action: usize) -> Value {
		self.0[state][action]
	}

	fn set(&mut self, state: usize, action: usize, value: Value) {
		self.0[state][action] = value;
	}
}


impl<const NUM_STATES: usize, const NUM_ACTIONS: usize> Default
	for QTable<{ NUM_STATES }, { NUM_ACTIONS }>
{
	fn default() -> Self { Self([[0.0; NUM_ACTIONS]; NUM_STATES]) }
}
/// A memoized QTable that stores the best action for each state.
/// Every time a value is set, the best action for that state is updated.
pub struct MemoizedQTable<const NUM_STATES: usize, const NUM_ACTIONS: usize> {
	table: QTable<{ NUM_STATES }, { NUM_ACTIONS }>,
	best_index: [usize; NUM_STATES],
}

impl<const NUM_STATES: usize, const NUM_ACTIONS: usize>
	MemoizedQTable<{ NUM_STATES }, { NUM_ACTIONS }>
{
	pub fn new() -> Self {
		Self {
			table: QTable::default(),
			best_index: [0; NUM_STATES],
		}
	}
	pub fn table(&self) -> &QTable<{ NUM_STATES }, { NUM_ACTIONS }> {
		&self.table
	}
	pub fn winners(&self) -> &[usize; NUM_STATES] { &self.best_index }
}

impl<const NUM_STATES: usize, const NUM_ACTIONS: usize> QSource
	for MemoizedQTable<{ NUM_STATES }, { NUM_ACTIONS }>
{
	const NUM_ACTIONS: usize = NUM_ACTIONS;
	fn greedy_policy(&self, state: usize) -> usize { self.best_index[state] }

	fn get(&self, state: usize, action: usize) -> Value {
		self.table.get(state, action)
	}
	fn set(&mut self, state: usize, action: usize, value: Value) {
		self.table.set(state, action, value);
		let mut max_value: Value = 0.;
		let mut max_index = 0;

		for (index, &value) in self.table[state].iter().enumerate() {
			if value > max_value {
				max_value = value;
				max_index = index;
			}
		}
		self.best_index[state] = max_index;
	}

	fn get_actions(&self, state: usize) -> impl Iterator<Item = &Value> {
		self.table.get_actions(state)
	}
}
