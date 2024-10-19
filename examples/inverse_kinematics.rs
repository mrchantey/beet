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
				beet_examples::robot_arm::create_render_camera,
				beet_examples::scenes::spatial::phone_screen,
				beet_examples::scenes::spatial::inverse_kinematics,
			),
		)
		.run();
}
