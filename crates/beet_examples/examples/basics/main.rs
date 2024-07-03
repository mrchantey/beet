//! In this example we will create an action
//! and then combine it with some built-in actions to run a behavior.
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;
mod scenes;

fn main() {
	let mut app = App::new();

	app.add_plugins((
		ExamplePluginText::default(),
		DefaultBeetPlugins::default(),
	))
	// .add_systems(Startup, scenes::hello_world)
	.add_systems(Startup, scenes::hello_net)
	.run();
}
