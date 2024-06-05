#![allow(dead_code)]
use super::QSource;
use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct QTableTrainer<M> {
	n_training_episodes: u32,
	learning_rate: f32,
	n_eval_episodes: u32,
	max_steps: u32,
	gamma: f32,
	eval_seed: Vec<u32>,
	max_epsilon: f32,
	min_epsilon: f32,
	decay_rate: f32,
	phantom: std::marker::PhantomData<M>,
}

impl<State, Action, Env, Table> Default
	for QTableTrainer<(State, Action, Env, Table)>
where
	State: StateSpace,
	Action: ActionSpace,
	Table: QSource<State = State, Action = Action>,
	Env: Environment<State = State, Action = Action>,
{
	fn default() -> Self { Self::new() }
}

impl<State, Action, Env, Table> QTableTrainer<(State, Action, Env, Table)>
where
	State: StateSpace,
	Action: ActionSpace,
	Table: QSource<State = State, Action = Action>,
	Env: Environment<State = State, Action = Action>,
{
	pub fn new() -> Self {
		Self {
			n_training_episodes: 10000,
			// n_training_episodes: 10,
			learning_rate: 0.7,
			n_eval_episodes: 100,
			max_steps: 99,
			gamma: 0.95,
			eval_seed: vec![],
			max_epsilon: 1.0,
			min_epsilon: 0.05,
			decay_rate: 0.0005,
			phantom: std::marker::PhantomData,
		}
	}

	pub fn train(
		&mut self,
		table: &mut Table,
		env: impl Fn() -> Env,
	) {
		for episode in 0..self.n_training_episodes {
			let epsilon = self.min_epsilon
				+ (self.max_epsilon - self.min_epsilon)
					* (-self.decay_rate * episode as f32).exp();

			let mut env = env();
			let mut prev_state = env.state();

			'step: for _step in 0..self.max_steps {
				let (action, _) =
					table.epsilon_greedy_policy(&prev_state, epsilon);
				let StepOutcome {
					state: new_state,
					reward,
					done,
				} = env.step(&action);
				// let new_state: usize = state.into();

				// Update Q(s,a):= Q(s,a) + lr [R(s,a) + gamma * max Q(s',a') - Q(s,a)]
				let prev_reward = table.get_q(&prev_state, &action);

				let (_, best_value) = table.greedy_policy(&new_state);
				// let (best_action, best_value) = table
				// 	.get_actions(&new_state)
				// 	.max_by(|(_, a), (_, b)| {
				// 		a.partial_cmp(b).unwrap_or(Ordering::Equal)
				// 	})
				// 	.unwrap();


				let discounted_reward = prev_reward
					+ self.learning_rate
						* (reward + self.gamma * best_value - prev_reward);

				table.set_q(&prev_state, &action, discounted_reward);
				prev_state = new_state;

				if done {
					break 'step;
				}
			}
		}
	}
	///   Evaluate the agent for ``n_eval_episodes`` episodes and returns average reward and std of reward.
	pub fn evaluate(
		&self,
		table: &Table,
		env: impl Fn() -> Env,
	) -> Evaluation {
		let mut rewards = Vec::new();
		for _episode in 0..self.n_training_episodes {
			let mut env = env();
			let mut prev_state = env.state();
			let mut total_reward = 0.0;

			for _step in 0..self.max_steps {
				let (action, _) = table.greedy_policy(&prev_state);
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
		Evaluation::new(rewards)
	}
}

#[derive(Debug, Clone)]
pub struct Evaluation {
	pub mean: f32,
	pub std: f32,
}

impl Evaluation {
	pub fn new(rewards: Vec<f32>) -> Self {
		let mean = mean(&rewards).unwrap();
		let std = variance(&rewards).unwrap();
		Self { mean, std }
	}
}

fn mean(data: &[f32]) -> Option<f32> {
	let len = data.len();
	if len == 0 {
		return None;
	}
	Some(data.iter().sum::<f32>() / len as f32)
}


fn variance(data: &[f32]) -> Option<f32> {
	let len = data.len();
	if len < 2 {
		return None;
	}

	let mean = data.iter().sum::<f32>() / len as f32;
	let var = data.iter().map(|value| (value - mean).powi(2)).sum::<f32>()
		/ (len - 1) as f32;
	Some(var)
}
