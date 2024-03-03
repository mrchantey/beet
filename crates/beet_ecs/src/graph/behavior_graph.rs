use crate::prelude::*;
use bevy_core::Name;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use petgraph::graph::DiGraph;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;

// pub type ActionList<T> = Vec<T>;
pub type BehaviorTree<T> = Tree<BehaviorNode<T>>;

#[derive(Default, Clone, Deref, DerefMut, Serialize, Deserialize)]
pub struct BehaviorGraph<T: ActionSuper>(pub DiGraph<BehaviorNode<T>, ()>);

impl<T: ActionSuper> PartialEq for BehaviorGraph<T> {
	fn eq(&self, other: &Self) -> bool { self.0.is_identical(other) }
}

impl<T: Debug + ActionSuper> Debug for BehaviorGraph<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
		// f.debug_tuple("BehaviorGraph").field(&self.0).finish()
	}
}


#[extend::ext]
pub impl<T: ActionSuper> BehaviorTree<T> {
	fn into_action_graph(self) -> BehaviorGraph<T> {
		BehaviorGraph(self.into_graph())
	}
}

impl<T: ActionSuper> BehaviorGraph<T> {
	pub fn new() -> Self { Self(DiGraph::new()) }
	pub fn from_tree(tree: BehaviorTree<T>) -> Self {
		Self(DiGraph::from_tree(tree))
	}

	pub fn with_indexed_names(mut self) -> Self {
		self.node_weights_mut().enumerate().for_each(|(i, node)| {
			node.name = format!("Node {i}");
		});
		self
	}


	pub fn spawn(
		&self,
		world: &mut impl WorldOrCommands,
		target: Entity,
	) -> EntityGraph {
		// create entities & actions
		let entity_graph = self.map(
			|_, actions| {
				let entity = world.spawn((
					Name::from("Action Graph Node"),
					TargetEntity(target),
					RunTimer::default(),
				));

				for action in actions.iter() {
					world.apply_action_typed(action, entity);
				}
				entity
			},
			|_, _| (),
		);

		// create edges
		for (index, entity) in Iterator::zip(
			entity_graph.node_indices(),
			entity_graph.node_weights(),
		) {
			let children = entity_graph
				.neighbors_directed_in_order(index, petgraph::Direction::Outgoing)
				.map(|index| entity_graph[index])
				.collect::<Vec<_>>();
			world.insert(*entity, Edges(children));
		}

		if let Some(root) = entity_graph.root() {
			world.insert(*root, Running);
		} else {
			// warn that graph is empty?
		}

		EntityGraph(entity_graph)
	}
}
