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
	// .add_systems(PostStartup, save_scene("target/scenes/hello_world.ron"))
	.run();
}