use beet::prelude::*;
use beet_examples::*;
use bevy::prelude::*;
use std::time::Duration;

const SCENE_SCALE: f32 = 1.;

fn main() {
	let mut app = App::new();
	app.add_plugins((
		ExamplePlugin3d { ground: false },
		DefaultBeetPlugins,
		FrozenLakePlugin,
	))
	.add_systems(Startup, setup);

	app.run();
}


fn setup(
	mut commands: Commands,
	assets: Res<FrozenLakeAssets>,
	asset_server: Res<AssetServer>,
) {
	// camera
	commands
		.spawn((CameraDistance::new(SCENE_SCALE), Camera3dBundle::default()));
	// scene
	let map = FrozenLakeMap::default_four_by_four();
	let grid_to_world = GridToWorld::from_frozen_lake_map(&map, SCENE_SCALE);

	spawn_frozen_lake_scene(&mut commands, &map, &grid_to_world, &assets, ());
	// agent




	let agent_grid_pos = map.agent_position();
	let agent_pos = grid_to_world.world_pos(*agent_grid_pos);
	let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

	let policy_handle =
		asset_server.load::<FrozenLakeQTable>("ml/frozen_lake_qtable.ron");

	commands
		.spawn((
			SceneBundle {
				scene: assets.character.clone(),
				transform: Transform::from_translation(agent_pos)
					.with_scale(object_scale),
				..default()
			},
			grid_to_world.clone(),
			agent_grid_pos,
			GridDirection::sample(),
		))
		.with_children(|parent| {
			let agent = parent.parent_entity();

			parent
				.spawn((Running, SequenceSelector, Repeat::default()))
				.with_children(|parent| {
					parent.spawn((
						TargetAgent(agent),
						ReadQPolicy::new(policy_handle),
					));
					parent.spawn((
						TranslateGrid::new(Duration::from_secs(1)),
						TargetAgent(agent),
						RunTimer::default(),
					));
				});
		});
}
