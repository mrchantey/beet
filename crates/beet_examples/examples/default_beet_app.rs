//! In this example we will create an action
//! and then combine it with some built-in actions to run a behavior.
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;
use anyhow::Result;

fn main() {
	let mut app = App::new();
	app.add_plugins(ExamplePluginFull::default());

	#[cfg(not(target_arch = "wasm32"))]
	load_scenes(app.world_mut()).unwrap();

	app.run();
}

#[allow(unused)]
fn load_scenes(world: &mut World)-> Result<()>{
	let args: Vec<String> = std::env::args().collect();

	// The first argument is the path to the program
	for argument in args.iter().skip(1) {
		println!("Loading scene from: {argument}");
		let scene = std::fs::read_to_string(argument).unwrap();
		write_ron_to_world(&scene, world).unwrap();
	}
	Ok(())
}
