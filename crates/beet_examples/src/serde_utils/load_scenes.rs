use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;

pub fn load_scenes_from_args(_world: &mut World) {
	#[cfg(not(target_arch = "wasm32"))]
	{
		let args: Vec<String> = std::env::args().collect();
		load_scenes(_world, args).expect("Error loading scenes from cli args");
	}
}

#[allow(unused)]
fn load_scenes(world: &mut World, args: Vec<String>) -> Result<()> {
	// The first argument is the path to the program
	for path in args.iter().skip(1) {
		let path = format!("target/scenes/{}.ron", path);
		log::info!("Loading scene from: {path}");
		let scene = std::fs::read_to_string(path)?;
		write_ron_to_world(&scene, world)?;
	}
	Ok(())
}
