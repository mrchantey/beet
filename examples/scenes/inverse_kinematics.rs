use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(
			Startup,
			(
				bevyhub::core::scenes::lighting_3d,
				bevyhub::core::scenes::ground_3d,
				beet_examples::scenes::spatial::spawn_ik_camera,
				beet_examples::scenes::spatial::spawn_arm_with_keyboard_target,
			),
		)
		.run();
}
