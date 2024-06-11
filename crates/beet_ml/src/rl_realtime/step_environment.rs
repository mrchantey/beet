use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct StepEnvironment<S: RlSessionTypes> {
	episode: u32,
	step: u32,
	phantom: PhantomData<S>,
}

impl<S: RlSessionTypes> StepEnvironment<S> {
	pub fn new(episode: u32) -> Self {
		Self {
			episode,
			step: 0,
			phantom: PhantomData,
		}
	}
}


fn step_environment<S: RlSessionTypes>(
	mut rng: ResMut<RlRng>,
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
	mut query: Query<
		(Entity, &TargetAgent, &mut StepEnvironment<S>),
		Added<Running>,
	>,
) where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	for (action_entity, agent, mut step) in query.iter_mut() {
		let Ok((state, mut action, mut env, params, session_entity)) =
			agents.get_mut(**agent)
		else {
			continue;
		};
		let Ok(mut table) = sessions.get_mut(**session_entity) else {
			continue;
		};

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

		commands.entity(action_entity).insert(RunResult::Success);
		step.step += 1;

		if outcome.done || step.step >= params.max_steps {
			end_episode_events.send(EndEpisode::new(**session_entity));
		}
	}
}

impl<S: RlSessionTypes> ActionMeta for StepEnvironment<S> {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<S: RlSessionTypes> ActionSystems for StepEnvironment<S>
where
	S::State: Component,
	S::Action: Component,
	S::QLearnPolicy: Component,
	S::Env: Component,
{
	fn systems() -> SystemConfigs { step_environment::<S>.in_set(TickSet) }
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

		app.add_plugins((
			LifecyclePlugin,
			ActionPlugin::<StepEnvironment<FrozenLakeQTableSession>>::default(),
			RlSessionPlugin::<FrozenLakeEpParams>::default(),
		))
		.insert_time();

		let map = FrozenLakeMap::default_four_by_four();

		let mut rng = RlRng::deterministic();

		let session = app.world_mut().spawn(FrozenLakeQTable::default()).id();

		let agent = app
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
				parent.spawn((
					TargetAgent(parent.parent_entity()),
					StepEnvironment::<FrozenLakeQTableSession>::new(0),
					Running,
				));
			})
			.id();

		app.world_mut().insert_resource(rng);

		let tree = EntityTree::new_with_world(agent, app.world_mut());

		expect(tree.component_tree(app.world()))
			.to_be(Tree::new(None).with_leaf(Some(&Running)))?;

		app.update();

		expect(tree.component_tree::<Running>(app.world()))
			.to_be(Tree::new(None).with_leaf(None))?;


		let table = app.world().get::<FrozenLakeQTable>(session).unwrap();
		expect(table.keys().next()).to_be(Some(&GridPos(UVec2::new(0, 0))))?;
		let inner = table.values().next().unwrap();
		expect(inner.iter().next().unwrap().1).to_be(&0.)?;

		Ok(())
	}
}
