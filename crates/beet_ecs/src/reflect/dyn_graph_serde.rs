use crate::prelude::*;
use bevy::ecs::entity::Entity;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::ecs::world::World;
use bevy::scene::serde::SceneDeserializer;
use bevy::scene::serde::SceneSerializer;
use bevy::scene::DynamicScene;
use serde::de::DeserializeSeed;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

#[derive(Serialize, Deserialize)]
pub struct DynGraphSerde<T: ActionTypes> {
	#[serde(bound = "")]
	scene: GraphSceneSerde<T>,
	root: Entity,
}

impl<T: ActionTypes> DynGraphSerde<T> {
	pub fn from_dyn_graph(graph: &DynGraph) -> Self {
		let scene = DynamicScene::from_world(&graph.world().read());
		Self {
			scene: GraphSceneSerde::new(scene),
			root: graph.root(),
		}
	}

	pub fn into_dyn_graph(&self) -> anyhow::Result<DynGraph> {
		let mut world = World::new();
		world.insert_resource(GraphSceneSerde::<T>::type_registry());
		// let dyn_graph = DynGraph::new(node)
		let mut entity_map = Default::default();
		self.scene
			.scene
			.write_to_world(&mut world, &mut entity_map)?;

		let root = entity_map[&self.root];
		let graph = DynGraph::new_with::<T>(world, root);
		Ok(graph)
	}
}

impl<T: ActionTypes> From<&DynGraph> for DynGraphSerde<T> {
	fn from(graph: &DynGraph) -> Self { Self::from_dyn_graph(graph) }
}

impl<T: ActionTypes> Into<DynGraph> for DynGraphSerde<T> {
	fn into(self) -> DynGraph { self.into_dyn_graph().unwrap() }
}

/// Basic serde functionality for a scene
pub struct GraphSceneSerde<T: ActionTypes> {
	pub scene: DynamicScene,
	phantom: PhantomData<T>,
}

impl<T: ActionTypes> GraphSceneSerde<T> {
	pub fn new(scene: DynamicScene) -> Self {
		Self {
			scene,
			phantom: PhantomData,
		}
	}
	pub fn type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		append_beet_type_registry_with_generics::<T>(&registry);
		registry
	}
}

impl<T: ActionTypes> Serialize for GraphSceneSerde<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let registry = Self::type_registry();
		let scene_serializer = SceneSerializer::new(&self.scene, &registry);
		scene_serializer.serialize(serializer)
	}
}

impl<'de, T: ActionTypes> Deserialize<'de> for GraphSceneSerde<T> {
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
	use super::DynGraphSerde;
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let graph1 = (Repeat, FallbackSelector)
			.child(EmptyAction)
			.into_dyn_graph::<EcsNode>();

		let serde1 = graph1.into_serde::<EcsNode>();
		let bin1 = bincode::serialize(&serde1)?;
		let serde2 = bincode::deserialize::<DynGraphSerde<EcsNode>>(&bin1)?;
		let graph2 = serde2.into_dyn_graph()?;


		let root2 = graph2.root();
		let world2 = graph2.world().read();
		let world2: &World = &world2;
		expect(world2).to_have_entity(root2)?;
		expect(world2).to_have_component::<Repeat>(root2)?;
		expect(world2).to_have_component::<FallbackSelector>(root2)?;
		let child = world2.get::<Edges>(root2).unwrap()[0];
		expect(world2).to_have_component::<EmptyAction>(child)?;

		let bin2 = bincode::serialize(&graph2.into_serde::<EcsNode>())?;
		let serde3 = bincode::deserialize::<DynGraphSerde<EcsNode>>(&bin2)?;
		let graph3 = serde3.into_dyn_graph()?;

		let root3 = graph3.root();
		let world3 = graph3.world().read();
		let world3: &World = &world3;
		expect(world3).to_have_entity(root3)?;
		expect(world2).to_have_component::<Repeat>(root3)?;
		Ok(())
	}
}
