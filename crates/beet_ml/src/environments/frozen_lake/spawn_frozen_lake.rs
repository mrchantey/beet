use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::time::Duration;


pub fn spawn_frozen_lake(
	mut events: EventReader<StartEpisode<FrozenLakeEpParams>>,
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	for event in events.read() {
		let map = FrozenLakeMap::default_four_by_four();

		let grid_to_world =
			GridToWorld::from_frozen_lake_map(&map, event.params.map_width);

		let tile_scale = Vec3::splat(grid_to_world.cell_width);
		let tile_handle = asset_server
			.load("kaykit-minigame/tileSmall_teamBlue.gltf.glb#Scene0");
		for x in 0..map.width() {
			for y in 0..map.height() {
				let mut pos = grid_to_world.world_pos(UVec2::new(x, y));
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

		let goal_handle = asset_server
			.load("kaykit-minigame/flag_teamYellow.gltf.glb#Scene0");

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
								params: event.params.learn_params.clone(),
								trainer: EpisodeOwner(trainer),
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
										TranslateGrid::new(
											Duration::from_secs(1),
										),
										TargetAgent(agent),
										RunTimer::default(),
									));
									parent.spawn((
										TargetAgent(agent),
										StepEnvironment::<
											FrozenLakeEnv,
											FrozenLakeQTable,
										>::new(event.episode),
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
}
