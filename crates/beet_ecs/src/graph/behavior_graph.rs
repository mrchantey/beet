use crate::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use petgraph::graph::DiGraph;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;

// pub type ActionList<T> = Vec<T>;

/// A directed [`petgraph::graph`] where each node is a [`BehaviorNode`].
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


impl<T: ActionSuper> BehaviorGraph<T> {
	pub fn new() -> Self { Self(DiGraph::new()) }
	pub fn from_tree(tree: BehaviorTree<T>) -> Self { tree.into() }

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
		EntityGraph::new(world, self.clone(), target)
	}
}
