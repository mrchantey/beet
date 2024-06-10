use crate::prelude::*;
use rand::Rng;

pub trait QSource: 'static + Send + Sync {
	type State: StateSpace;
	type Action: ActionSpace;


	fn step(
		&mut self,
		params: &QLearnParams,
		rng: &mut impl Rng,
		epsilon: f32,
		action: &Self::Action,
		prev_state: &Self::State,
		state: &Self::State,
		reward: f32,
	) -> Self::Action {
		self.set_discounted_reward(params, action, reward, prev_state, state);
		let (action, _) = self.epsilon_greedy_policy(&state, epsilon, rng);
		action
	}

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
			(Self::Action::sample_with_rng(rng), QValue::default())
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


	fn set_discounted_reward(
		&mut self,
		params: &QLearnParams,
		action: &Self::Action,
		reward: QValue,
		prev_state: &Self::State,
		next_state: &Self::State,
	) {
		let prev_q = self.get_q(&prev_state, &action);
		let (_, new_max_q) = self.greedy_policy(&next_state);

		// Bellman equation
		// Q(s,a):= Q(s,a) + lr [R(s,a) + gamma * max Q(s',a') - Q(s,a)]
		let discounted_reward = prev_q
			+ params.learning_rate
				* (reward + params.gamma * new_max_q - prev_q);

		self.set_q(&prev_state, &action, discounted_reward);
	}
}
