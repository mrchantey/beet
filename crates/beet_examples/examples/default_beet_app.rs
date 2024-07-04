//! In this example we will create an action
//! and then combine it with some built-in actions to run a behavior.
use anyhow::Result;
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins((
		ExampleDefaultPlugins,
		DefaultBeetPlugins,
		ExamplePlugins,
	));

	#[cfg(not(target_arch = "wasm32"))]
	load_scenes(app.world_mut()).unwrap();

	app.run();
}

#[allow(unused)]
fn load_scenes(world: &mut World) -> Result<()> {
	let args: Vec<String> = std::env::args().collect();

	// The first argument is the path to the program
	for path in args.iter().skip(1) {
		let path = format!("target/scenes/beet-basics/{}.ron", path);
		log::info!("Loading scene from: {path}");
		let scene = std::fs::read_to_string(path).unwrap();
		write_ron_to_world(&scene, world).unwrap();
	}
	Ok(())
}
