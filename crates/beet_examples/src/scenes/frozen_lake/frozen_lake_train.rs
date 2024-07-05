use beet::prelude::*;
use bevy::prelude::*;
use super::*;

pub fn frozen_lake_train(mut commands: Commands) {

	let map = FrozenLakeMap::default_four_by_four();
	let params = FrozenLakeEpParams {
		learn_params: default(),
		grid_to_world: GridToWorld::from_frozen_lake_map(&map, FROZEN_LAKE_SCENE_SCALE),
		map,
	};
	commands.spawn((RlSession::new(params), FrozenLakeQTable::default()));
}
