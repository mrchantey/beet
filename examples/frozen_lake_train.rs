//! # Frozen Lake Training
//!
//! Rendering a reinforcement learning algorithm can be entertaining and useful for debugging.
use beet::prelude::*;
use beet_examples::*;
use bevy::prelude::*;

const SCENE_SCALE: f32 = 1.;

fn main() {
	let mut app = App::new();
	app.add_plugins((
		ExamplePlugin3d { ground: false },
		DefaultBeetPlugins,
		FrozenLakePlugin,
	))
	.add_systems(Startup, (setup_camera, setup_runner));

	app.run();
}


fn setup_camera(mut commands: Commands) {
	commands
		.spawn((CameraDistance::new(SCENE_SCALE), Camera3dBundle::default()));
}


fn setup_runner(mut commands: Commands) {
	let map = FrozenLakeMap::default_four_by_four();
	let params = FrozenLakeEpParams {
		learn_params: default(),
		grid_to_world: GridToWorld::from_frozen_lake_map(&map, SCENE_SCALE),
		map,
	};
	commands.spawn((RlSession::new(params), FrozenLakeQTable::default()));
}
