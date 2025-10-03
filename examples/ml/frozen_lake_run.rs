use beet::examples::scenes;
use beet::examples::scenes::ml::FROZEN_LAKE_SCENE_SCALE;
use beet::prelude::*;

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

fn setup(
	mut commands: Commands,
	mut rng: ResMut<RandomSource>,
	asset_server: Res<AssetServer>,
) {
	let map = FrozenLakeMap::default_four_by_four();
	let grid_to_world =
		GridToWorld::from_frozen_lake_map(&map, FROZEN_LAKE_SCENE_SCALE);

	// agent
	let agent_grid_pos = map.agent_position();
	let agent_pos = grid_to_world.world_pos(*agent_grid_pos);
	let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

	let qtable =
		asset_server.load::<FrozenLakeQTable>("ml/frozen_lake_qtable.ron");

	commands.spawn((
		Name::new("Inference Agent"),
		SceneRoot(asset_server.load(frozen_lake_assets::CHARACTER)),
		Transform::from_translation(agent_pos).with_scale(object_scale),
		grid_to_world.clone(),
		agent_grid_pos,
		GridDirection::sample(&mut rng.0),
		children![(
			RunOnAssetReady::new(qtable.clone()),
			Sequence::default(),
			Repeat::default(),
			Name::new("Run Frozen Lake Agent"),
			children![
				(
					Name::new("Get next action"),
					HandleWrapper(qtable),
					ReadQPolicy::<FrozenLakeQTable>::default(),
				),
				(
					Name::new("Perform action"),
					TranslateGrid::new(Duration::from_secs(1)),
				)
			]
		)],
	));
}
