use crate::beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;

pub const FROZEN_LAKE_SCENE_SCALE: f32 = 1.;


pub fn frozen_lake_scene(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
) {
	commands.spawn((
		Camera3d::default(),
		CameraDistance::new(FROZEN_LAKE_SCENE_SCALE * 0.7),
	));

	let map = FrozenLakeMap::default_four_by_four();
	let grid_to_world =
		GridToWorld::from_frozen_lake_map(&map, FROZEN_LAKE_SCENE_SCALE);

	let tile_scale = Vec3::splat(grid_to_world.cell_width);
	for x in 0..map.num_cols() {
		for y in 0..map.num_rows() {
			let mut pos = grid_to_world.world_pos(UVec2::new(x, y));
			pos.y -= grid_to_world.cell_width;
			commands.spawn((
				Transform::from_translation(pos).with_scale(tile_scale),
				SceneRoot(asset_server.load(frozen_lake_assets::TILE)),
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
					Transform::from_translation(pos).with_scale(object_scale),
					SceneRoot(asset_server.load(frozen_lake_assets::HAZARD)),
				));
			}
			FrozenLakeCell::Goal => {
				commands.spawn((
					SceneRoot(asset_server.load(frozen_lake_assets::GOAL)),
					Transform::from_translation(pos).with_scale(object_scale),
				));
			}
			FrozenLakeCell::Ice => { /* already spawned on the grid */ }
			FrozenLakeCell::Agent => { /*spawns on episode */ }
		}
		{}
	}
}
