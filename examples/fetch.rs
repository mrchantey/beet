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
		.add_plugins(ExamplePluginBasics)
		.add_systems(
			Startup,
			(
				// scenes::beet_debug,
				scenes::ui_terminal_input,
				scenes::lighting_3d,
				scenes::ground_3d,
				scenes::fetch_scene,
				scenes::fetch_npc,
			),
		)
		.run();
}
