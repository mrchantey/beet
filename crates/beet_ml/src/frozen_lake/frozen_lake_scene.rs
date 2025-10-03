use crate::prelude::*;
use beet_core::prelude::*;

pub fn spawn_frozen_lake_scene(
	mut commands: Commands,
	map: &FrozenLakeMap,
	grid_to_world: &GridToWorld,
	asset_server: Res<AssetServer>,
) {
	let tile_scale = Vec3::splat(grid_to_world.cell_width);
	for x in 0..map.num_cols() {
		for y in 0..map.num_rows() {
			let mut pos = grid_to_world.world_pos(UVec2::new(x, y));
			pos.y -= grid_to_world.cell_width;
			commands.spawn((
				SceneRoot(asset_server.load(frozen_lake_assets::TILE)),
				Transform::from_translation(pos).with_scale(tile_scale),
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
					SceneRoot(asset_server.load(frozen_lake_assets::HAZARD)),
					Transform::from_translation(pos).with_scale(object_scale),
				));
			}
			FrozenLakeCell::Goal => {
				commands.spawn((
					SceneRoot(asset_server.load(frozen_lake_assets::GOAL)),
					Transform::from_translation(pos).with_scale(object_scale),
				));
			}
			FrozenLakeCell::Ice => {}
			FrozenLakeCell::Agent => { /*spawns on episode */ }
		}
		{}
	}
}
