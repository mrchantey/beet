//! # Hello ML
//! A popular 'hello world' for machine learning in games is sentence similarity,
//! where models rank the similarity of sentences.
//! This example uses a locally run *small* language model to select the child behavior with the most similar sentence.
use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn main() {
	App::new()
		.add_plugins(ExamplePluginFull)
		.add_systems(
			Startup,
			(
				scenes::beet_debug,
				scenes::camera_2d,
				scenes::ui_terminal,
				scenes::hello_ml,
			),
		)
		.run();
}

/*
STDOUT:

Started: Sentence Selector
Started: Attack Behavior

*/
