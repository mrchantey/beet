use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(running_beet_example_plugin)
		.add_systems(
			Startup,
			(
				beet_examples::scenes::spatial::inverse_kinematics,
			),
		)
		.run();
}
