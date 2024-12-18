//! Fetch is a combined example demonstrating the following behaviors:
//! - Machine Learning
//! - Animation
//! - UI
//!
//! Please wait for the status to change to `Idle` before issuing commands.
//!
use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((running_beet_example_plugin, plugin_ml))
		.add_systems(
			Startup,
			(
				bevyhub::core::scenes::ui_terminal_input,
				bevyhub::core::scenes::lighting_3d,
				bevyhub::core::scenes::ground_3d,
				beet_examples::scenes::flow::beet_debug,
				beet_examples::scenes::ml::fetch_scene,
				beet_examples::scenes::ml::fetch_npc,
			),
		)
		.run();
}