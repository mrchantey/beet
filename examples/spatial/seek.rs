use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(
			Startup,
			(
				bevyhub::core::scenes::camera_2d,
				bevyhub::core::scenes::space_scene,
				beet_examples::scenes::flow::beet_debug,
				beet_examples::scenes::spatial::seek,
			),
		)
		.run();
}
