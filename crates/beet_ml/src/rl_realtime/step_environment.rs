use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use std::marker::PhantomData;

#[action(step_environment::<S>)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct StepEnvironment<S: RlSessionTypes>
where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	episode: u32,
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
	pub fn new(episode: u32) -> Self {
		Self {
			episode,
			step: 0,
			phantom: PhantomData,
		}
	}
}


fn step_environment<S: RlSessionTypes>(
	ev: On<GetOutcome>,
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
	mut query: Query<&mut StepEnvironment<S>>,
) -> Result
where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	let action_entity = ev.target();
	let mut step = query.get_mut(action_entity)?;
	let (state, mut action, mut env, params, session_entity) =
		agents.get_mut(action_entity)?;
	let mut table = sessions.get_mut(**session_entity)?;

	let outcome = env.step(&state, &action);
	// we ignore the state of the outcome, allow simulation to determine
	let epsilon = params.epsilon(step.episode);

	*action = table.step(
		params,
		&mut **rng,
		epsilon,
		&action,
		state,
		&outcome.state,
		outcome.reward,
	);
	// log::info!(
	// 	"step complete - action: {:?}, reward: {:?}",
	// 	action,
	// 	outcome.reward
	// );
	commands.entity(action_entity).trigger_target(Outcome::Pass);
	step.step += 1;

	if outcome.done || step.step >= params.max_steps {
		end_episode_events.write(EndEpisode::new(**session_entity));
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();

		let on_result =
			observer_ext::observe_triggers::<Outcome>(app.world_mut());

		app.add_plugins((
			ControlFlowPlugin::default(),
			RlSessionPlugin::<FrozenLakeEpParams>::default(),
		))
		.init_resource::<RandomSource>()
		.insert_time();

		let map = FrozenLakeMap::default_four_by_four();

		let mut rng = RandomSource::from_seed(0);

		let session = app.world_mut().spawn(FrozenLakeQTable::default()).id();

		app.world_mut()
			.spawn((
				RlAgentBundle {
					state: map.agent_position(),
					action: GridDirection::sample(&mut *rng),
					env: QTableEnv::new(map.transition_outcomes()),
					params: QLearnParams::default(),
					session: SessionEntity(session),
					despawn: DespawnOnEpisodeEnd,
				},
				StepEnvironment::<FrozenLakeQTableSession>::new(0),
			))
			.trigger_target(GetOutcome)
			.flush();


		app.world_mut().insert_resource(rng);

		app.update();

		(on_result.len() > 0).xpect_true();


		let table = app.world().get::<FrozenLakeQTable>(session).unwrap();
		table
			.keys()
			.next()
			.xpect_eq(Some(&GridPos(UVec2::new(0, 0))));
		let inner = table.values().next().unwrap();
		inner.iter().next().unwrap().1.xpect_eq(0.);
	}
}
