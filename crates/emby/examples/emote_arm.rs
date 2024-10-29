use beet_examples::prelude::*;
use bevy::prelude::*;
use emby::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((crate_test_beet_example_plugin, plugin_ml, EmbyPlugin))
		.add_systems(
			Startup,
			(
				beetmash::core::scenes::lighting_3d,
				beetmash::core::scenes::ground_3d,
				beetmash::core::scenes::ui_terminal_input,
				// beet_examples::scenes::flow::beet_debug_start_and_stop,
				emby::scenes::emote_arm_camera,
				emby::scenes::emote_arm,
				emby::scenes::spawn_barbarian,
				emby::scenes::phone_texture_camera_3d,
			),
		)
		.run();
}
