use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;
use sweet::prelude::RandomSource;

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
	ev: Trigger<OnRun>,
	mut rng: ResMut<RandomSource>,
	mut end_episode_events: EventWriter<EndEpisode<S::EpisodeParams>>,
	mut commands: Commands,
	mut sessions: Query<&mut S::QLearnPolicy>,
	mut agents: Query<(
		&S::State,
		&mut S::Action,
		&mut S::Env,
		&QLearnParams,
		&SessionEntity,
	)>,
	mut query: Query<&mut StepEnvironment<S>>,
) where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	let mut step = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	let (state, mut action, mut env, params, session_entity) = agents
		.get_mut(ev.origin)
		.expect(&expect_action::to_have_origin(&ev));
	let mut table = sessions
		.get_mut(**session_entity)
		.expect(&expect_action::to_have_other(&ev));

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
	ev.trigger_result(&mut commands, RunResult::Success);
	step.step += 1;

	if outcome.done || step.step >= params.max_steps {
		end_episode_events.write(EndEpisode::new(**session_entity));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_flow::prelude::*;
	use beet_utils::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();

		let on_result =
			observer_ext::observe_triggers::<OnResult>(app.world_mut());

		app.add_plugins((
			BeetFlowPlugin::default(),
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
			.flush_trigger(OnRun::local());


		app.world_mut().insert_resource(rng);

		app.update();

		(on_result.len() > 0).xpect_true();


		let table = app.world().get::<FrozenLakeQTable>(session).unwrap();
		table
			.keys()
			.next()
			.xpect_eq(Some(&GridPos(UVec2::new(0, 0))));
		let inner = table.values().next().unwrap();
		inner.iter().next().unwrap().1.xpect_eq(&0.);
	}
}
