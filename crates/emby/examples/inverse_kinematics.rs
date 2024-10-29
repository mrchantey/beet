use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(
			Startup,
			(
				beetmash::core::scenes::lighting_3d,
				beetmash::core::scenes::ground_3d,
				emby::scenes::spawn_ik_camera,
				emby::scenes::spawn_arm_with_keyboard_target,
				emby::scenes::phone_texture_emoji,
				emby::scenes::phone_texture_camera_2d,
			),
		)
		.run();
}
