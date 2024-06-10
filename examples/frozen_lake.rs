use beet::prelude::*;
use beet_examples::*;
use bevy::prelude::*;
use std::time::Duration;

const MAP_WIDTH: f32 = 4.;

fn main() {
	let mut app = App::new();
	app.add_plugins((
		ExamplePlugin3d { ground: false },
		DefaultBeetPlugins,
		FrozenLakePlugin,
	))
	.add_systems(Startup, (setup_camera, setup_environment));


	app.run();
}


fn setup_camera(mut commands: Commands) {
	commands.spawn((
		CameraDistance {
			width: MAP_WIDTH * 1.1,
			offset: Vec3::new(0., 4., 4.),
		},
		Camera3dBundle::default(),
	));
}



fn setup_environment(mut commands: Commands, asset_server: Res<AssetServer>) {
	let map = FrozenLakeMap::default_four_by_four();

	let grid_to_world = GridToWorld::from_frozen_lake_map(&map, MAP_WIDTH);

	let tile_scale = Vec3::splat(grid_to_world.cell_width);
	let tile_handle =
		asset_server.load("kaykit-minigame/tileSmall_teamBlue.gltf.glb#Scene0");
	for x in 0..map.width() {
		for y in 0..map.height() {
			let mut pos =
				grid_to_world.world_pos(UVec2::new(x as u32, y as u32));
			pos.y -= grid_to_world.cell_width;
			commands.spawn((SceneBundle {
				scene: tile_handle.clone(),
				transform: Transform::from_translation(pos)
					.with_scale(tile_scale),
				..default()
			},));
		}
	}
	// if let Some(agent_pos) = map.agent_position() {
	// 	let pos =
	// 		offset + Vec3::new(agent_pos.x as f32, 0.1, agent_pos.y as f32);
	// }

	let character_handle =
		asset_server.load("kaykit-minigame/character_dog.gltf.glb#Scene0");

	let goal_handle =
		asset_server.load("kaykit-minigame/flag_teamYellow.gltf.glb#Scene0");

	let hazard_handle =
		asset_server.load("kaykit-minigame/bomb_teamRed.gltf.glb#Scene0");


	let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

	for (index, cell) in map.cells().iter().enumerate() {
		let grid_pos = map.index_to_position(index);
		let mut pos = grid_to_world.world_pos(grid_pos);
		match cell {
			FrozenLakeCell::Agent => {
				let trainer = commands.spawn_empty().id();


				commands
					.spawn((
						SceneBundle {
							scene: character_handle.clone(),
							transform: Transform::from_translation(pos)
								.with_scale(object_scale),
							..default()
						},
						grid_to_world.clone(),
						RlAgentBundle {
							state: map.agent_position(),
							action: GridDirection::sample(),
							table: QTable::default(),
							env: FrozenLakeEnv::new(map.clone(), false),
							params: QLearnParams::default(),
							trainer: Trainer(trainer),
						},
					))
					.with_children(|parent| {
						let agent = parent.parent_entity();

						parent
							.spawn((
								Running,
								SequenceSelector,
								Repeat::default(),
							))
							.with_children(|parent| {
								parent.spawn((
									TranslateGrid::new(Duration::from_secs(1)),
									TargetAgent(agent),
									RunTimer::default(),
								));
								parent.spawn((
									TargetAgent(agent),
									StepEnvironment::<FrozenLakeEnv, FrozenLakeQTable>::new(0),
								));
							});
					});
			}
			FrozenLakeCell::Hole => {
				pos.y += grid_to_world.cell_width * 0.25; // this asset is a bit too low
				commands.spawn(SceneBundle {
					scene: hazard_handle.clone(),
					transform: Transform::from_translation(pos)
						.with_scale(object_scale),
					..default()
				});
			}
			FrozenLakeCell::Goal => {
				commands.spawn(SceneBundle {
					scene: goal_handle.clone(),
					transform: Transform::from_translation(pos)
						.with_scale(object_scale),
					..default()
				});
			}
			FrozenLakeCell::Ice => {}
		}
		{}
	}
}
