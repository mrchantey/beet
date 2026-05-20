//! Implementation of the OpenAI Gym Frozen Lake environment.
//! https://github.com/openai/gym/blob/master/gym/envs/toy_text/frozen_lake.py
//!
//! Frozen lake involves crossing a frozen lake from Start(S) to Goal(G)
//! without falling into any Holes(H) by walking over the Frozen(F) lake.
//! The agent may not always move in the intended direction due to the
//! slippery nature of the frozen lake.
//!
//! ### Action Space
//! The agent takes a 1-element vector for actions. The action space is
//! `(dir)`, where `dir` decides direction to move in which can be:
//!
//! - 0: LEFT
//! - 1: DOWN
//! - 2: RIGHT
//! - 3: UP
//!
//! ### Observation Space
//! The observation is a value representing the agent's current position
//! as `current_row * nrows + current_col` (both 0-indexed). For a 4x4
//! map the goal is at `3 * 4 + 3 = 15`. The observation count is
//! determined by map size.
//!
//! ### Rewards
//! - Reach Goal(G): +1
//! - Reach Hole(H): 0
//! - Reach Frozen(F): 0
mod frozen_lake_map;
pub use self::frozen_lake_map::*;
mod frozen_lake_scene;
pub use self::frozen_lake_scene::*;
mod grid;
pub use self::grid::*;
mod reward_grid;
pub use self::reward_grid::*;
mod spawn_frozen_lake;
pub use self::spawn_frozen_lake::*;
mod translate_grid;
pub use self::translate_grid::*;

use crate::PostTickSet;
use crate::TickSet;
use crate::prelude::*;
use beet_core::prelude::*;

/// Registers frozen-lake assets, session machinery and the per-tick
/// systems that translate the agent across the grid and award rewards.
pub struct FrozenLakePlugin;

impl Plugin for FrozenLakePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(RlSessionPlugin::<FrozenLakeEpParams>::default())
			.add_systems(
				Update,
				(
					translate_grid.in_set(TickSet),
					step_environment::<FrozenLakeQTableSession>.in_set(TickSet),
					reward_grid.in_set(PostTickSet),
					spawn_frozen_lake_episode.in_set(PostTickSet),
				),
			)
			.init_asset::<QTable<GridPos, GridDirection>>()
			.init_asset_loader::<QTableLoader<GridPos, GridDirection>>()
			.register_type::<GridPos>()
			.register_type::<GridDirection>()
			.register_type::<GridToWorld>()
			.register_type::<RlSession<FrozenLakeEpParams>>()
			.register_type::<QTable<GridPos, GridDirection>>();

		let world = app.world_mut();
		world.register_component::<GridPos>();
		world.register_component::<GridDirection>();
	}
}

/// Parameters for a single frozen-lake training session.
#[derive(Debug, Clone, Reflect)]
pub struct FrozenLakeEpParams {
	/// Q-learning hyperparameters.
	pub learn_params: QLearnParams,
	/// The map being trained on.
	pub map: FrozenLakeMap,
	/// Mapping from grid coordinates to world space, used by visualisers.
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

/// QTable specialised to the frozen-lake (GridPos, GridDirection) pair.
pub type FrozenLakeQTable = QTable<GridPos, GridDirection>;

/// Concrete [`RlSessionTypes`] bundle for the frozen-lake demo.
#[derive(Debug, Reflect)]
pub struct FrozenLakeQTableSession;

impl RlSessionTypes for FrozenLakeQTableSession {
	type State = GridPos;
	type Action = GridDirection;
	type QLearnPolicy = FrozenLakeQTable;
	type Env = QTableEnv<Self::State, Self::Action>;
	type EpisodeParams = FrozenLakeEpParams;
}
