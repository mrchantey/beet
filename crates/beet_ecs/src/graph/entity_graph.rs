use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashSet;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;

#[derive(Debug, Default, Clone, Deref, DerefMut, Component)]
pub struct EntityGraph(pub DiGraph<Entity, ()>);

impl EntityGraph {
	pub fn from_world(world: &World, root: Entity) -> Self {
		let mut this = EntityGraph::default();
		this.add_recursive(world, root, &mut Default::default());
		this
	}
	fn add_recursive(
		&mut self,
		world: &World,
		parent: Entity,
		visited: &mut HashSet<Entity>,
	) -> Option<NodeIndex> {
		if visited.contains(&parent) {
			return None;
		}
		visited.insert(parent);

		let node_index = self.add_node(parent);
		if let Some(children) = world.get::<Edges>(parent) {
			for child in children.iter() {
				if let Some(child_index) =
					self.add_recursive(world, *child, visited)
				{
					self.add_edge(node_index, child_index, ());
				}
			}
		}
		Some(node_index)
	}


	pub fn despawn(&self, commands: &mut Commands) {
		for entity in self.node_weights() {
			commands.entity(*entity).despawn();
		}
	}
}
