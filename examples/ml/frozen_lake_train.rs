use beet::examples::scenes;
use beet::examples::scenes::ml::FROZEN_LAKE_SCENE_SCALE;
use beet::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((running_beet_example_plugin, plugin_ml))
		.add_systems(
			Startup,
			(
				scenes::ui_terminal,
				scenes::lighting_3d,
				scenes::ml::frozen_lake_scene,
				setup,
			),
		)
		.run();
}

fn setup(mut commands: Commands) {
	let map = FrozenLakeMap::default_four_by_four();
	let params = FrozenLakeEpParams {
		learn_params: default(),
		grid_to_world: GridToWorld::from_frozen_lake_map(
			&map,
			FROZEN_LAKE_SCENE_SCALE,
		),
		map,
	};
	commands.spawn((RlSession::new(params), FrozenLakeQTable::default()));
}
