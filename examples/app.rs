//! For use with scene-based workflows
//!
//! Usage:
//! 1. build scenes: `cargo run -p beet_examples --example build_scenes`
//! 2. run: `cargo run --example defaylt_beet_app <scene_names>`
//!
//! Common combinations:
//! - hello world: 	`beet-debug camera-2d ui-terminal hello-world`
//! - hello_llm: 		`beet-debug camera-2d ui-terminal sentence-selector`
//! - seek: 				`beet-debug camera-2d space-scene seek`
//! - flocking: 		`beet-debug camera-2d space-scene flock`
//! - animation:		`beet-debug animation-demo`
//!
use anyhow::Result;
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins(ExamplePluginFull)
	/*-*/;

	#[cfg(not(target_arch = "wasm32"))]
	load_scenes(app.world_mut()).unwrap();

	app.run();
}

#[allow(unused)]
fn load_scenes(world: &mut World) -> Result<()> {
	let args: Vec<String> = std::env::args().collect();

	// The first argument is the path to the program
	for path in args.iter().skip(1) {
		let path = format!("target/scenes/{}.ron", path);
		log::info!("Loading scene from: {path}");
		let scene = std::fs::read_to_string(path).unwrap();
		write_ron_to_world(&scene, world).unwrap();
	}
	Ok(())
}
