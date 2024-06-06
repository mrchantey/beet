use crate::prelude::*;
use bevy::prelude::*;
use rand::rngs::StdRng;

#[derive(Component)]
pub struct QTableRunner<S: QSource> {
	params: Readonly<QLearnParams>,
	step: u32,
	episode: u32,
	pub source: S,
	rng: StdRng,
	state: S::State,
}

impl<S: QSource> QTableRunner<S> {
	pub fn new(
		params: QLearnParams,
		source: S,
		state: S::State,
		rng: StdRng,
	) -> Self {
		Self {
			step: 0,
			episode: 0,
			source,
			rng,
			state,
			params: Readonly::new(params),
		}
	}

	pub fn epsilon(&self) -> f32 { self.params.next_epsilon(self.episode) }

	pub fn episodes_finished(&self) -> bool {
		self.episode >= self.params.n_training_episodes
	}

	pub fn next_episode(&mut self, state: S::State) {
		self.step = 0;
		self.episode += 1;
		self.state = state;
	}

	fn next_action(&mut self) -> S::Action {
		let action = self
			.source
			.epsilon_greedy_policy(&self.state, self.epsilon(), &mut self.rng)
			.0;
		action
	}

	pub fn step(
		&mut self,
		action: &S::Action,
		state: S::State,
		reward: f32,
	) -> S::Action {
		self.source.set_discounted_reward(
			&self.params,
			action,
			reward,
			&self.state,
			&state,
		);
		self.step += 1;
		self.state = state;
		self.next_action()
	}

	pub fn steps_finished(&self) -> bool { self.step >= self.params.max_steps }
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use rand::rngs::StdRng;
	use rand::SeedableRng;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let source = QTable::<GridPos, GridDirection>::default();
		let env = FrozenLakeEnv::default();
		let initial_state = env.state();

		let mut runner = QTableRunner::new(
			QLearnParams::default(),
			source,
			initial_state.clone(),
			StdRng::seed_from_u64(0),
		);

		while !runner.episodes_finished() {
			let mut env = FrozenLakeEnv::default();
			let mut action = runner.next_action();

			while !runner.steps_finished() {
				let outcome = env.step(&action);
				// Must step even if outcome is done, to remember reward
				action = runner.step(&action, outcome.state, outcome.reward);
				if outcome.done {
					break;
				}
			}
			runner.next_episode(initial_state.clone());
		}

		let eval = QTableTrainer::new(FrozenLakeEnv::default(), runner.source)
			.evaluate();

		expect(eval.mean).to_be(1.)?;
		expect(eval.std).to_be(0.)?;
		expect(eval.total_steps).to_be(600)?;


		Ok(())
	}
}
