use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(
			Startup,
			(
				beet_examples::scenes::flow::beet_debug,
				beetmash::core::scenes::camera_2d,
				beetmash::core::scenes::ui_terminal,
				beet_examples::scenes::flow::hello_world,
			),
		)
		.run();
}
