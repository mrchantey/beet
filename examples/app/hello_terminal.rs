use beet::examples::scenes;
use beet::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.init_resource::<DebugOnRun>()
		.init_resource::<DebugToStdOut>()
		.add_systems(
			Startup,
			(
				scenes::camera_2d,
				scenes::ui_terminal_input,
				scenes::hello_world,
			),
		)
		.run();
}
