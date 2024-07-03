use anyhow::Result;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::scene::serde::SceneDeserializer;
use forky_core::ResultTEExt;
use serde::de::DeserializeSeed;
use serde::Deserialize;
use serde::Serialize;
/// Received by this app, containing the raw text of a ron file for
/// deserialization and spawning
#[derive(Debug, Clone, Serialize, Deserialize, Event, Reflect)]
pub struct OnSpawnScene(pub String);

pub fn handle_spawn_scene(
	world: &mut World,
	events: &mut SystemState<EventReader<OnSpawnScene>>,
) {
	events
		.get_mut(world)
		.read()
		.map(|e| e.0.clone())
		.collect::<Vec<_>>()
		.into_iter()
		.map(|scene| spawn(&scene, world))
		.collect::<Result<Vec<_>>>()
		.ok_or(|e| log::error!("{e}"));
}

fn spawn(ron_str: &str, world: &mut World) -> Result<()> {
	let type_registry = world.resource::<AppTypeRegistry>().clone();
	let mut deserializer =
		bevy::scene::ron::de::Deserializer::from_str(ron_str)?;
	let scene_deserializer = SceneDeserializer {
		type_registry: &type_registry.read(),
	};
	let scene = scene_deserializer
		.deserialize(&mut deserializer)
		.map_err(|e| deserializer.span_error(e))?;
	let mut entity_map = Default::default();
	scene.write_to_world(world, &mut entity_map)?;
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::log::LogPlugin;
	use bevy::prelude::*;
	use sweet::*;

	#[derive(Debug, Component, Reflect, PartialEq)]
	#[reflect(Component)]
	struct MyStruct(pub u32);


	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.register_type::<MyStruct>();

		app.world_mut().spawn(MyStruct(7));
		let scene = DynamicScene::from_world(app.world());
		let str = scene
			.serialize(&app.world().resource::<AppTypeRegistry>().read())?;

		let mut app2 = App::new();

		app2.add_plugins((LogPlugin::default(), ReplicatePlugin, CommonEventsPlugin))
			.add_systems(Update, handle_spawn_scene).register_type::<MyStruct>();

		app2.world_mut().send_event(OnSpawnScene(str.into()));

		expect(
			app2.world_mut()
				.query::<&MyStruct>()
				.iter(app2.world())
				.count(),
		)
		.to_be(0)?;
		app2.update();
		expect(
			app2.world_mut()
				.query::<&MyStruct>()
				.iter(app2.world())
				.next(),
		)
		.to_be(Some(&MyStruct(7)))?;

		Ok(())
	}
}
