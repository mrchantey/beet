use crate::prelude::*;
use sweet::prelude::*;

pub trait QPolicy: 'static + Send + Sync {
	type State: StateSpace;
	type Action: ActionSpace;


	fn step(
		&mut self,
		params: &QLearnParams,
		rng: &mut impl Rng,
		epsilon: f32,
		action: &Self::Action,
		state: &Self::State,
		// the anticipated state, it may not occur
		next_state: &Self::State,
		reward: f32,
	) -> Self::Action {
		self.set_discounted_reward(params, action, reward, state, next_state);
		let (action, _) = self.epsilon_greedy_policy(&next_state, epsilon, rng);
		action
	}

	fn set_discounted_reward(
		&mut self,
		params: &QLearnParams,
		action: &Self::Action,
		reward: QValue,
		state: &Self::State,
		// the anticipated state, it may not occur
		next_state: &Self::State,
	) {
		let prev_q = self.get_q(&state, &action);
		let (_, new_max_q) = self.greedy_policy(&next_state);

		// Bellman equation
		// Q(s,a):= Q(s,a) + lr [R(s,a) + gamma * max Q(s',a') - Q(s,a)]
		let discounted_reward = prev_q
			+ params.learning_rate
				* (reward + params.gamma * new_max_q - prev_q);

		self.set_q(&state, &action, discounted_reward);
	}

	fn greedy_policy(&self, state: &Self::State) -> (Self::Action, QValue);
	fn epsilon_greedy_policy(
		&self,
		state: &Self::State,
		epsilon: f32,
		rng: &mut impl Rng,
	) -> (Self::Action, QValue) {
		let random_num: f32 = rng.r#gen(); // generates a float between 0 and 1
		// let random_num: f32 = rng.random(); // generates a float between 0 and 1
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
