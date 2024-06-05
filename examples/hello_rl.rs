//! This is a 'hello world' of reinforcement learning using the well-known `frozen_lake` environment.
//! It uses a 'non-deep' technique called Q-Tables to learn the optimal policy.
use beet::prelude::*;
use beet_examples::*;
use bevy::prelude::*;

const CELL_WIDTH: f32 = 1.;

fn main() {
	let mut app = App::new();
	app.add_plugins((ExamplePlugin3d, DefaultBeetPlugins, MlPlugin))
		.add_systems(Startup, (setup_camera, setup_environment));


	app.run();
}


fn setup_camera(mut commands: Commands) {
	commands.spawn((
		// camera always in line with front row of items
		// and keeps them exactly in view
		CameraDistance {
			x: CELL_WIDTH * 1.6,
			origin: Vec3::new(0., 0., CELL_WIDTH),
		},
		Camera3dBundle {
			transform: Transform::from_xyz(0., 1.6, 5.)
				.looking_at(Vec3::new(0., 0., 0.), Vec3::Y),
			..default()
		},
	));
}

fn setup_environment(){






}