use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

/**
Implementation of the OpenAI Gym Frozen Lake environment.
https://github.com/openai/gym/blob/master/gym/envs/toy_text/frozen_lake.py

Frozen lake involves crossing a frozen lake from Start(S) to Goal(G) without falling into any Holes(H)
by walking over the Frozen(F) lake.
The agent may not always move in the intended direction due to the slippery nature of the frozen lake.

### Action Space
The agent takes a 1-element vector for actions.
The action space is `(dir)`, where `dir` decides direction to move in which can be:

- 0: LEFT
- 1: DOWN
- 2: RIGHT
- 3: UP

### Observation Space
The observation is a value representing the agent's current position as
current_row * nrows + current_col (where both the row and col start at 0).
For example, the goal position in the 4x4 map can be calculated as follows: 3 * 4 + 3 = 15.
The number of possible observations is dependent on the size of the map.
For example, the 4x4 map has 16 possible observations.

### Rewards

Reward schedule:
- Reach goal(G): +1
- Reach hole(H): 0
- Reach frozen(F): 0
**/
pub struct FrozenLakePlugin;

impl Plugin for FrozenLakePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			ActionPlugin::<(
				TranslateGrid,
				StepEnvironment<FrozenLakeQTableSession>,
				ReadQPolicy<FrozenLakeQTable>,
			)>::default(),
			RlSessionPlugin::<FrozenLakeEpParams>::default(),
		))
		.add_systems(PreStartup, init_frozen_lake_assets)
		.add_systems(Update, reward_grid.in_set(PostTickSet))
		.add_systems(
			Update,
			(spawn_frozen_lake_session, spawn_frozen_lake_episode)
				.in_set(PostTickSet),
		)
		.init_resource::<RlRng>()
		.init_asset::<QTable<GridPos, GridDirection>>()
		.init_asset_loader::<QTableLoader<GridPos, GridDirection>>();

		let world = app.world_mut();
		world.init_component::<GridPos>();
		world.init_component::<GridDirection>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();

		registry.register::<GridPos>();
		registry.register::<GridDirection>();
	}
}


#[derive(Debug, Clone, Reflect)]
pub struct FrozenLakeEpParams {
	pub learn_params: QLearnParams,
	pub map: FrozenLakeMap,
	pub grid_to_world: GridToWorld,
}

impl Default for FrozenLakeEpParams {
	fn default() -> Self {
		let map = FrozenLakeMap::default_four_by_four();
		Self {
			learn_params: QLearnParams::default(),
			grid_to_world: GridToWorld::from_frozen_lake_map(&map, 4.0),
			map,
		}
	}
}

impl EpisodeParams for FrozenLakeEpParams {
	fn num_episodes(&self) -> u32 { self.learn_params.n_training_episodes }
}

pub type FrozenLakeQTable = QTable<GridPos, GridDirection>;

#[derive(Debug, Reflect)]
pub struct FrozenLakeQTableSession;

impl RlSessionTypes for FrozenLakeQTableSession {
	type State = GridPos;
	type Action = GridDirection;
	type QLearnPolicy = FrozenLakeQTable;
	type Env = QTableEnv<Self::State, Self::Action>;
	type EpisodeParams = FrozenLakeEpParams;
}
