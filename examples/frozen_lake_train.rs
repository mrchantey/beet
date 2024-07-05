use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(ExamplePluginFull)
		.add_systems(
			Startup,
			(
				scenes::beet_debug,
				scenes::ui_terminal,
				scenes::lighting_3d,
				scenes::frozen_lake::frozen_lake_scene,
				scenes::frozen_lake::frozen_lake_train,
			),
		)
		.run();
}
