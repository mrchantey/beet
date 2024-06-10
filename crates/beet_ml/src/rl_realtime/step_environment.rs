use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct StepEnvironment<
	Env: Component + Environment<State = Table::State, Action = Table::Action>,
	Table: Component + QSource,
> {
	episode: u32,
	step: u32,
	phantom: PhantomData<(Env, Table)>,
}

impl<
		Env: Component + Environment<State = Table::State, Action = Table::Action>,
		Table: Component + QSource,
	> StepEnvironment<Env, Table>
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
	Env: Component + Environment<State = Table::State, Action = Table::Action>,
	Table: Component + QSource,
>(
	mut rng: ResMut<RlRng>,
	mut commands: Commands,
	mut agents: Query<(
		&Table::State,
		&mut Table::Action,
		&mut Table,
		&mut Env,
		&QLearnParams,
		&EpisodeOwner,
	)>,
	mut query: Query<
		(Entity, &TargetAgent, &mut StepEnvironment<Env, Table>),
		Added<Running>,
	>,
) {
	for (action_entity, agent, mut step) in query.iter_mut() {
		log::info!("step start");
		let Ok((state, mut action, mut table, mut env, params, trainer)) =
			agents.get_mut(**agent)
		else {
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
		log::info!(
			"step complete - action: {:?}, reward: {:?}",
			action,
			outcome.reward
		);

		commands.entity(action_entity).insert(RunResult::Success);

		step.step += 1;
		if outcome.done || step.step >= params.max_steps {
			commands.entity(**trainer).insert(RunResult::Success);
			log::info!("episode complete");
		}
	}
}

impl<
		Env: Component + Environment<State = Table::State, Action = Table::Action>,
		Table: Component + QSource,
	> ActionMeta for StepEnvironment<Env, Table>
{
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<
		Env: Component + Environment<State = Table::State, Action = Table::Action>,
		Table: Component + QSource,
	> ActionSystems for StepEnvironment<Env, Table>
{
	fn systems() -> SystemConfigs {
		step_environment::<Env, Table>.in_set(TickSet)
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

		app.add_plugins((LifecyclePlugin, FrozenLakePlugin))
			.insert_time();

		let map = FrozenLakeMap::default_four_by_four();

		let mut rng = RlRng::deterministic();

		let trainer = app.world_mut().spawn_empty().id();

		let agent = app
			.world_mut()
			.spawn(RlAgentBundle {
				state: map.agent_position(),
				action: GridDirection::sample_with_rng(&mut *rng),
				table: QTable::default(),
				env: FrozenLakeEnv::new(map, false),
				params: QLearnParams::default(),
				trainer: EpisodeOwner(trainer),
			})
			.with_children(|parent| {
				parent.spawn((
					TargetAgent(parent.parent_entity()),
					StepEnvironment::<FrozenLakeEnv, FrozenLakeQTable>::new(0),
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


		let table = app.world().get::<FrozenLakeQTable>(agent).unwrap();
		expect(table.keys().next()).to_be(Some(&GridPos(UVec2::new(0, 0))))?;
		let inner = table.values().next().unwrap();
		expect(inner.iter().next().unwrap())
			.to_be((&GridDirection::Left, &0.))?;

		Ok(())
	}
}
