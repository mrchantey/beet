//! This is a 'hello world' of reinforcement learning using the well-known `frozen_lake` environment.
//! It uses a 'non-deep' technique called Q-Tables to learn the optimal policy.
use beet::prelude::*;
use beet_examples::*;
use bevy::prelude::*;

const CELL_WIDTH: f32 = 1.;

fn main() {
	let mut app = App::new();
	app.add_plugins((ExamplePlugin3d::default(), DefaultBeetPlugins, MlPlugin))
		.add_systems(Startup, (setup_camera, setup_environment));


	app.run();
}


fn setup_camera(mut commands: Commands) {
	commands.spawn((
		CameraDistance {
			width: CELL_WIDTH * 1.6,
			offset: Vec3::new(0., 1.6, CELL_WIDTH),
		},
		Camera3dBundle::default(),
	));
}

fn setup_environment() {}
