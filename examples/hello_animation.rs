use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(ExamplePluginBasics)
		.add_systems(
			Startup,
			(
				scenes::beet_debug,
				scenes::ui_terminal,
				scenes::lighting_3d,
				scenes::ground_3d,
				scenes::hello_animation,
			),
		)
		.run();
}
