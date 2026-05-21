use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use std::marker::PhantomData;

/// Long-running action that ticks the environment one step per frame.
///
/// Each frame while [`Running`] it samples an action via epsilon-greedy,
/// queries [`Environment::step`], discounts the reward into the session's
/// [`QPolicy`], and advances state. When the environment returns
/// `done == true` or the per-episode step budget is exhausted, the action
/// resolves [`Outcome::PASS`] and an [`EndEpisode`] message is queued.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ContinueRun)]
pub struct StepEnvironment<S: RlSessionTypes>
where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	/// Episode index, used for epsilon decay.
	episode: u32,
	/// Steps taken so far in the current episode.
	step: u32,
	#[reflect(ignore)]
	phantom: PhantomData<S>,
}

impl<S: RlSessionTypes> StepEnvironment<S>
where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	/// Create a [`StepEnvironment`] for the given episode index.
	pub fn new(episode: u32) -> Self {
		Self {
			episode,
			step: 0,
			phantom: PhantomData,
		}
	}
}

/// Per-frame system: takes one environment step for every active
/// [`StepEnvironment`] with [`Running`].
pub fn step_environment<S: RlSessionTypes>(
	mut rng: ResMut<RandomSource>,
	mut end_episode_events: MessageWriter<EndEpisode<S::EpisodeParams>>,
	mut commands: Commands,
	mut sessions: Query<&mut S::QLearnPolicy>,
	mut agents: AgentQuery<(
		&S::State,
		&mut S::Action,
		&mut S::Env,
		&QLearnParams,
		&SessionEntity,
	)>,
	mut query: Query<(Entity, &mut StepEnvironment<S>), With<Running>>,
) -> Result
where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	for (action_entity, mut step) in query.iter_mut() {
		let (state, mut action, mut env, params, session_entity) =
			agents.get_mut(action_entity)?;
		let mut table = sessions.get_mut(**session_entity)?;
		// 1. step the environment (state of the outcome is ignored — the
		//    simulation determines the actually-reached state separately)
		let outcome = env.step(&state, &action);
		let epsilon = params.epsilon(step.episode);
		// 2. fold reward into the policy and pick the next action
		*action = table.step(
			params,
			&mut **rng,
			epsilon,
			&action,
			state,
			&outcome.state,
			outcome.reward,
		);
		step.step += 1;
		// 3. on episode end, complete the run and notify
		if outcome.done || step.step >= params.max_steps {
			end_episode_events.write(EndEpisode::new(**session_entity));
			commands.entity(action_entity).queue(EndRun(Outcome::PASS));
		}
	}
	Ok(())
}
