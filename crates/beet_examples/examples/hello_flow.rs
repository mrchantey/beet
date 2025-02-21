use beet::prelude::*;
use beet_examples::prelude::*;
use beet_examples::scenes;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.init_resource::<DebugOnRun>()
		.add_systems(
			Startup,
			(
				scenes::camera_2d,
				scenes::ui_terminal,
				// scenes::flow::beet_debug,
				scenes::hello_world,
			),
		)
		.run();
}
