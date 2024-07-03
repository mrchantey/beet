use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use std::fs::File;
use std::fs::{
	self,
};
use std::io::Write;


#[derive(Component)]
pub struct SerializeMarker;

// fn entities_to_serialize(world: &mut World) -> Vec<Entity> {
// 	world
// 		.query_filtered::<Entity, With<SerializeMarker>>()
// 		.iter(world)
// 		.collect::<Vec<_>>()
// 		.into_iter()
// 		.map(|parent| ChildrenExt::collect_world(parent, world))
// 		.flatten()
// 		.collect()
// }


pub fn save_scene(filename: impl Into<String>) -> SystemConfigs {
	let filename = filename.into();
	(move |world: &mut World| {
		let filename = filename.clone();
		let scene = DynamicScene::from_world(world);
		// let entities = entities_to_serialize(world);
		// let scene = DynamicSceneBuilder::from_world(world)
		// 	.extract_entities(entities.into_iter())
		// 	.build();

		// Scenes can be serialized like this:
		let type_registry = world.resource::<AppTypeRegistry>();
		let type_registry = type_registry.read();
		let serialized_scene = scene.serialize(&type_registry).unwrap();

		// Showing the scene in the console
		// info!("{}", serialized_scene);

		// Writing the scene to a new file. Using a task to avoid calling the filesystem APIs in a system
		// as they are blocking
		// This can't work in WASM as there is no filesystem access
		#[cfg(not(target_arch = "wasm32"))]
		IoTaskPool::get()
			.spawn(async move {
				let dir_path = std::path::Path::new(&filename).parent().unwrap();
				fs::create_dir_all(dir_path)
					.expect("Error while creating directory");

				// Write the scene RON data to file
				File::create(filename)
					.and_then(|mut file| {
						file.write(serialized_scene.as_bytes())
					})
					.expect("Error while writing scene to file");
			})
			.detach();
	})
	.into_configs()
}
