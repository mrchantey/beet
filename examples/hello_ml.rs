//! # Hello ML
//! A popular 'hello world' for machine learning in games is sentence similarity,
//! where models rank the similarity of sentences.
//! This example uses a locally run *small* language model to select the child behavior with the most similar sentence.
use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins((running_beet_example_plugin, plugin_ml))
		.add_systems(
			Startup,
			(
				beetmash::core::scenes::camera_2d,
				beetmash::core::scenes::ui_terminal_input,
				beet_examples::scenes::flow::beet_debug,
				beet_examples::scenes::ml::hello_ml,
			),
		)
		.run();
}
