use crate::prelude::*;
use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;
use bevy::scene::DynamicScene;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct DynGraphSerde<T: ActionTypes> {
	#[serde(bound = "")]
	scene: BeetSceneSerde<T>,
	root: Entity,
}

impl<T: ActionTypes> DynGraphSerde<T> {
	pub fn from_dyn_graph(graph: &DynGraph) -> Self {
		let scene = DynamicScene::from_world(&graph.world().read());
		Self {
			scene: BeetSceneSerde::new(scene),
			root: graph.root(),
		}
	}

	pub fn into_dyn_graph(&self) -> anyhow::Result<DynGraph> {
		let mut world = World::new();
		world.insert_resource(BeetSceneSerde::<T>::type_registry());
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
