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
				beet_examples::emote_agent::scenes::inverse_kinematics,
				beet_examples::emote_agent::scenes::phone_texture,
				beet_examples::emote_agent::scenes::phone_texture_camera,
			),
		)
		.run();
}
