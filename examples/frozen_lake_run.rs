use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((running_beet_example_plugin, plugin_ml))
		.add_systems(
			Startup,
			(
				// bevyhub::core::scenes::ui_terminal,
				bevyhub::core::scenes::lighting_3d,
				// beet_examples::scenes::flow::beet_debug,
				beet_examples::scenes::ml::frozen_lake_scene,
				beet_examples::scenes::ml::frozen_lake_run,
			),
		)
		.run();
}
