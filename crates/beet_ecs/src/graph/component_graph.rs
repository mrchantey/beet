use crate::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_ecs::world::World;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use std::fmt::Debug;


#[derive(Debug, Clone, Deref, DerefMut)]
pub struct ComponentGraph<'a, T>(pub DiGraph<Option<&'a T>, ()>);

impl<'a, T: Debug> ComponentGraph<'a, T> {
	pub fn print_tree(&self) {
		for node in self.0.node_indices() {
			println!("{:?}", self.0.node_weight(node));
		}
	}
}


impl<'a, T: Component> ComponentGraph<'a, T> {
	pub fn from_world(world: &'a World, entity_graph: &EntityGraph) -> Self {
		let component_graph =
			entity_graph.map(|_, entity| world.get::<T>(*entity), |_, _| ());
		Self(component_graph)
	}
	pub fn from_query(
		query: &'a Query<&T>,
		entity_graph: &EntityGraph,
	) -> Self {
		let component_graph =
			entity_graph.map(|_, entity| query.get(*entity).ok(), |_, _| ());
		Self(component_graph)
	}

	pub fn from_edges(root: Entity, world: &'a World) -> Self {
		let mut this = Self(DiGraph::default());
		this.add_recursive(root, world);
		this
	}
	fn add_recursive(&mut self, entity: Entity, world: &'a World) -> NodeIndex {
		let value = world.get::<T>(entity);
		let node_index = self.add_node(value);
		if let Some(children) = world.get::<Edges>(entity) {
			for child in children.iter() {
				let child_index = self.add_recursive(*child, world);
				self.add_edge(node_index, child_index, ());
			}
		}
		node_index
	}

	//eww, deprecate
	pub fn index(
		entity: Entity,
		world: &'a World,
		index: usize,
	) -> Option<&'a T> {
		ComponentGraph::<T>::from_edges(entity, world)
			.node_weight(NodeIndex::new(index))
			.unwrap()
			.as_ref()
			.copied()
	}
}

impl<'a, T: Component> ComponentGraph<'a, T>
where
	T: Clone,
{
	pub fn copy_nodes(&self) -> DiGraph<Option<T>, ()> {
		let mut graph = DiGraph::default();
		for node in self.0.node_indices() {
			graph.add_node(self.0.node_weight(node).unwrap().cloned());
		}
		for edge in self.0.edge_indices() {
			let (a, b) = self.0.edge_endpoints(edge).unwrap();
			graph.add_edge(a, b, ());
		}
		graph
	}
}
