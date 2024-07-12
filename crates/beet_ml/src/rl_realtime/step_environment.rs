use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(step_environment::<S>)]
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
	trigger: Trigger<OnRun>,
	mut rng: Option<ResMut<RlRng>>,
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
	mut query: Query<(&TargetAgent, &mut StepEnvironment<S>)>,
) where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	let (agent, mut step) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);
	let Ok((state, mut action, mut env, params, session_entity)) =
		agents.get_mut(**agent)
	else {
		return;
	};
	let Ok(mut table) = sessions.get_mut(**session_entity) else {
		return;
	};

	let outcome = env.step(&state, &action);
	// we ignore the state of the outcome, allow simulation to determine
	let epsilon = params.epsilon(step.episode);

	let rng = if let Some(rng) = &mut rng {
		rng.as_mut()
	} else {
		&mut RlRng::default()
	};

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

	commands.trigger_targets(OnRunResult::success(), trigger.entity());
	step.step += 1;

	if outcome.done || step.step >= params.max_steps {
		end_episode_events.send(EndEpisode::new(**session_entity));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();

		let on_result = observe_triggers::<OnRunResult>(app.world_mut());

		app.add_plugins((
			LifecyclePlugin,
			ActionPlugin::<StepEnvironment<FrozenLakeQTableSession>>::default(),
			RlSessionPlugin::<FrozenLakeEpParams>::default(),
		))
		.insert_time();

		let map = FrozenLakeMap::default_four_by_four();

		let mut rng = RlRng::deterministic();

		let session = app.world_mut().spawn(FrozenLakeQTable::default()).id();

		app
			.world_mut()
			.spawn(RlAgentBundle {
				state: map.agent_position(),
				action: GridDirection::sample_with_rng(&mut *rng),
				env: QTableEnv::new(map.transition_outcomes()),
				params: QLearnParams::default(),
				session: SessionEntity(session),
				despawn: DespawnOnEpisodeEnd,
			})
			.with_children(|parent| {
				parent
					.spawn((
						TargetAgent(parent.parent_entity()),
						StepEnvironment::<FrozenLakeQTableSession>::new(0),
					))
					.flush_trigger(OnRun);
			});


		app.world_mut().insert_resource(rng);

		app.update();

		expect(&on_result).to_have_been_called()?;


		let table = app.world().get::<FrozenLakeQTable>(session).unwrap();
		expect(table.keys().next()).to_be(Some(&GridPos(UVec2::new(0, 0))))?;
		let inner = table.values().next().unwrap();
		expect(inner.iter().next().unwrap().1).to_be(&0.)?;

		Ok(())
	}
}
