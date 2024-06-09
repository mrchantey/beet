use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::marker::PhantomData;

/// Used for training a QTable to completion with a provided [`Environment`].
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct StepEnvironment<
	S: StateSpace,
	A: ActionSpace,
	Env: Component + Environment<State = S, Action = A>,
	Table: Component + QSource<State = S, Action = A>,
> {
	episode: u32,
	step: u32,
	phantom: PhantomData<(S, A, Env, Table)>,
}

impl<
		S: StateSpace,
		A: ActionSpace,
		Env: Component + Environment<State = S, Action = A>,
		Table: Component + QSource<State = S, Action = A>,
	> StepEnvironment<S, A, Env, Table>
{
	pub fn new(episode: u32) -> Self {
		Self {
			episode,
			step: 0,
			phantom: PhantomData,
		}
	}
}


fn step_environment<
	S: StateSpace,
	A: ActionSpace,
	Env: Component + Environment<State = S, Action = A>,
	Table: Component + QSource<State = S, Action = A>,
>(
	mut commands: Commands,
	mut agents: Query<(
		&S,
		&mut A,
		&mut Table,
		&mut Env,
		&QLearnParams,
		&Trainer,
	)>,
	mut query: Query<
		(Entity, &TargetAgent, &mut StepEnvironment<S, A, Env, Table>),
		Added<Running>,
	>,
) {
	for (action_entity, agent, mut step) in query.iter_mut() {
		let Ok((state, mut action, mut table, mut env, params, trainer)) =
			agents.get_mut(**agent)
		else {
			continue;
		};
		let mut rng = StdRng::from_entropy();

		let outcome = env.step(&state, &action);
		// we ignore the state of the outcome, allow simulation to determine
		let epsilon = params.epsilon(step.episode);

		*action = table.step(
			params,
			&mut rng,
			epsilon,
			&action,
			state,
			&outcome.state,
			outcome.reward,
		);

		commands.entity(action_entity).insert(RunResult::Success);
		
		step.step += 1;
		if outcome.done || step.step >= params.max_steps {
			commands.entity(**trainer).insert(RunResult::Success);
		}
	}
}

impl<
		S: StateSpace,
		A: ActionSpace,
		Env: Component + Environment<State = S, Action = A>,
		Table: Component + QSource<State = S, Action = A>,
	> ActionMeta for StepEnvironment<S, A, Env, Table>
{
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<
		S: StateSpace,
		A: ActionSpace,
		Env: Component + Environment<State = S, Action = A>,
		Table: Component + QSource<State = S, Action = A>,
	> ActionSystems for StepEnvironment<S, A, Env, Table>
{
	fn systems() -> SystemConfigs {
		step_environment::<S, A, Env, Table>.in_set(TickSet)
	}
}
