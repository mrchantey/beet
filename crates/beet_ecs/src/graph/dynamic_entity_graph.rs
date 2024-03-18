use crate::prelude::*;
use bevy::prelude::*;
use bevy::scene::DynamicEntity;
use petgraph::graph::DiGraph;
use std::fmt;







#[derive(Default, Deref, DerefMut, Component)]
pub struct DynamicEntityGraph(pub DiGraph<DynamicEntity, ()>);


impl fmt::Debug for DynamicEntityGraph {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("DynamicEntityGraph")
			.field("graph", &"todo")
			.finish()
	}
}

impl Clone for DynamicEntityGraph {
	fn clone(&self) -> Self {
		DynamicEntityGraph(self.0.map(
			|_, e| DynamicEntity {
				entity: e.entity,
				components:
					e.components.iter().map(|c| c.clone_value()).collect(),
			},
			|_, _| (),
		))
	}
}


impl DynamicEntityGraph {
	pub fn new(world: &impl IntoWorld, root: Entity) -> Self {
		let world = world.into_world_ref();
		let entity_graph = EntityGraph::from_world(world, root);
		Self::from_entity_graph(world, entity_graph)
	}
	pub fn from_entity_graph(
		world: &impl IntoWorld,
		entity_graph: EntityGraph,
	) -> Self {
		let world = world.into_world_ref();
		DynamicEntityGraph(entity_graph.map(
			|_, entity| DynamicEntity {
				components: reflect_entity(world, *entity),
				entity: *entity,
			},
			|_, _| (),
		))
	}
	pub fn into_tree(self) -> Tree<DynamicEntity> { self.0.into_tree() }
}
