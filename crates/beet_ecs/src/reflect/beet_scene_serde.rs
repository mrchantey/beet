use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::scene::serde::SceneDeserializer;
use bevy::scene::serde::SceneSerializer;
use bevy::utils::hashbrown::HashSet;
use serde::de::DeserializeSeed;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

/// Basic serde functionality for a scene
pub struct BeetSceneSerde<T: ActionTypes> {
	pub scene: DynamicScene,
	phantom: PhantomData<T>,
}

impl<T: ActionTypes> BeetSceneSerde<T> {
	/// Creates a [`DynamicScene`] from the world, including all entities
	/// but no resources
	pub fn new(world: &World) -> Self {
		let registry = Self::type_registry();
		let registry = registry.read();
		let items = registry
			.iter()
			.map(|r| r.type_info().type_id())
			.collect::<HashSet<_>>();
		// let names = registry
		// 	.iter()
		// 	.map(|r| r.type_info().type_path_table().short_path())
		// 	.collect::<Vec<_>>()
		// 	.join("\n");
		// log::info!("Allowed types: {}", names);

		let scene = DynamicSceneBuilder::from_world(world)
			.deny_all_resources()
			.with_filter(SceneFilter::Allowlist(items))
			.extract_entities(world.iter_entities().map(|entity| entity.id()))
			.extract_resources()
			.build();
		Self::new_with(scene)
	}


	pub fn new_with(scene: DynamicScene) -> Self {
		Self {
			scene,
			phantom: PhantomData,
		}
	}

	pub fn type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		T::register_types(&mut registry.write());
		registry
	}
}

impl<T: ActionTypes> Serialize for BeetSceneSerde<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let registry = Self::type_registry();
		let registry = registry.read();
		let scene_serializer = SceneSerializer::new(&self.scene, &registry);
		scene_serializer.serialize(serializer)
	}
}

impl<'de, T: ActionTypes> Deserialize<'de> for BeetSceneSerde<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let registry = Self::type_registry();
		let scene_deserializer = SceneDeserializer {
			type_registry: &registry.read(),
		};
		let scene = scene_deserializer.deserialize(deserializer)?;

		Ok(Self {
			scene,
			phantom: PhantomData,
		})
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use bevy::time::TimePlugin;
	use sweet::*;


	#[derive(Reflect, Component)]
	struct MyStruct;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();
		world.insert_resource(BeetSceneSerde::<EcsModule>::type_registry());
		let entity = world
			.spawn((EmptyAction, Name::new("billy"), MyStruct))
			.id();

		let serde = BeetSceneSerde::<EcsModule>::new(&world);
		let bin = bincode::serialize(&serde)?;
		let serde = bincode::deserialize::<BeetSceneSerde<EcsModule>>(&bin)?;

		let mut world2 = World::new();
		world2.insert_resource(BeetSceneSerde::<EcsModule>::type_registry());

		let mut hashmap = Default::default();
		serde.scene.write_to_world(&mut world2, &mut hashmap)?;
		let entity = hashmap[&entity];

		expect(world2.entities().len()).to_be(1)?;

		expect(&world2).to_have_component::<EmptyAction>(entity)?;
		expect(&world2)
			.component(entity)?
			.to_be(&Name::new("billy"))?;
		expect(&world2)
			.not()
			.to_have_component::<MyStruct>(entity)?;

		Ok(())
	}

	#[test]
	fn works_with_app() -> Result<()> {
		let mut app = App::new();


		app /*-*/
		.add_plugins(TimePlugin)
		/*-*/;

		let world = app.world_mut();
		let entity = world.spawn_empty().id();
		let tree = test_serde_tree();
		tree.spawn(world, entity);
		let scene = BeetSceneSerde::<EcsModule>::new(world);
		let bin = bincode::serialize(&scene)?;
		let scene2 = bincode::deserialize::<BeetSceneSerde<EcsModule>>(&bin)?;
		let bin2 = bincode::serialize(&scene2)?;
		expect(bin).to_be(bin2)?;

		Ok(())
	}
}
