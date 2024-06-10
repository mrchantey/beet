use beet::prelude::*;
use beet_examples::*;
use bevy::prelude::*;

const MAP_WIDTH: f32 = 4.;

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
	commands.spawn((
		CameraDistance {
			width: MAP_WIDTH * 1.1,
			offset: Vec3::new(0., 4., 4.),
		},
		Camera3dBundle::default(),
	));
}


fn setup_runner(mut commands: Commands) {
	let params = FrozenLakeEpParams {
		learn_params: default(),
		map_width: MAP_WIDTH,
	};
	commands.spawn(EpisodeRunner::new(params));
}
