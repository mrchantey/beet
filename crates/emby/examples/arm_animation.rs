use beet_examples::prelude::*;
use bevy::prelude::*;
use emby::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((crate_test_beet_example_plugin, EmbyPlugin))
		.add_systems(
			Startup,
			(
				bevyhub::core::scenes::lighting_3d,
				bevyhub::core::scenes::ground_3d,
				beet_examples::scenes::flow::beet_debug_start_and_stop,
				beet_examples::scenes::spatial::spawn_ik_camera,
				emby::scenes::emote_arm,
			),
		)
		.run();
}
