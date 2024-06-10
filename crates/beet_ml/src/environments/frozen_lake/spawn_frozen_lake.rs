use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::time::Duration;


#[derive(Resource)]
pub struct FrozenLakeAssets {
	pub tile: Handle<Scene>,
	pub character: Handle<Scene>,
	pub goal: Handle<Scene>,
	pub hazard: Handle<Scene>,
}

pub fn init_frozen_lake_assets(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	let tile =
		asset_server.load("kaykit-minigame/tileSmall_teamBlue.gltf.glb#Scene0");
	let character =
		asset_server.load("kaykit-minigame/character_dog.gltf.glb#Scene0");
	let goal =
		asset_server.load("kaykit-minigame/flag_teamYellow.gltf.glb#Scene0");
	let hazard =
		asset_server.load("kaykit-minigame/bomb_teamRed.gltf.glb#Scene0");

	commands.insert_resource(FrozenLakeAssets {
		tile,
		character,
		goal,
		hazard,
	});
}

pub fn spawn_frozen_lake_static(
	mut events: EventReader<StartSession<FrozenLakeEpParams>>,
	mut commands: Commands,
	assets: Res<FrozenLakeAssets>,
) {
	for event in events.read() {
		let map = FrozenLakeMap::default_four_by_four();

		let grid_to_world =
			GridToWorld::from_frozen_lake_map(&map, event.params.map_width);

		let tile_scale = Vec3::splat(grid_to_world.cell_width);
		for x in 0..map.width() {
			for y in 0..map.height() {
				let mut pos = grid_to_world.world_pos(UVec2::new(x, y));
				pos.y -= grid_to_world.cell_width;
				commands.spawn((
					SceneBundle {
						scene: assets.tile.clone(),
						transform: Transform::from_translation(pos)
							.with_scale(tile_scale),
						..default()
					},
					EpisodeOwner(event.trainer),
				));
			}
		}

		let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

		for (index, cell) in map.cells().iter().enumerate() {
			let grid_pos = map.index_to_position(index);
			let mut pos = grid_to_world.world_pos(grid_pos);
			match cell {
				FrozenLakeCell::Hole => {
					pos.y += grid_to_world.cell_width * 0.25; // this asset is a bit too low
					commands.spawn((
						SceneBundle {
							scene: assets.hazard.clone(),
							transform: Transform::from_translation(pos)
								.with_scale(object_scale),
							..default()
						},
						EpisodeOwner(event.trainer),
					));
				}
				FrozenLakeCell::Goal => {
					commands.spawn((
						SceneBundle {
							scene: assets.goal.clone(),
							transform: Transform::from_translation(pos)
								.with_scale(object_scale),
							..default()
						},
						EpisodeOwner(event.trainer),
					));
				}
				FrozenLakeCell::Ice => {}
				FrozenLakeCell::Agent => { /*spawns on episode */ }
			}
			{}
		}
	}
}


pub fn spawn_frozen_lake(
	mut events: EventReader<StartEpisode<FrozenLakeEpParams>>,
	mut commands: Commands,
	assets: Res<FrozenLakeAssets>,
) {
	for event in events.read() {
		// TODO deduplicate
		let map = FrozenLakeMap::default_four_by_four();
		let grid_to_world =
			GridToWorld::from_frozen_lake_map(&map, event.params.map_width);
		let object_scale = Vec3::splat(grid_to_world.cell_width * 0.5);

		for (index, cell) in map.cells().iter().enumerate() {
			let grid_pos = map.index_to_position(index);
			let pos = grid_to_world.world_pos(grid_pos);
			match cell {
				FrozenLakeCell::Agent => {
					commands
						.spawn((
							SceneBundle {
								scene: assets.character.clone(),
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
								trainer: EpisodeOwner(event.trainer),
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
											FrozenLakeQTableSession,
										>::new(event.episode),
									));
								});
						});
				}
				_ => {}
			}
			{}
		}
	}
}
