use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(ExamplePluginFull)
		.add_systems(
			Startup,
			(
				scenes::beet_debug,
				scenes::camera_2d,
				scenes::space_scene,
				scenes::seek,
			),
		)
		.run();
}
