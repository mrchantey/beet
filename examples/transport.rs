use beet::prelude::*;
use beet_examples::*;
use bevy::prelude::*;

fn main() {
	App::new()
		.add_transport(WebEventClient::new_with_window())
		.add_plugins((ExamplePlugin,  ExampleReplicatePlugin))
		.run();
}
