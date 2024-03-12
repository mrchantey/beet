use crate::prelude::*;
use bevy_ecs::entity::Entity;


#[derive(Clone)]
pub struct BehaviorTree(pub Tree<BehaviorNode>);



impl BehaviorTree {
	pub fn new<M>(item: impl IntoBehaviorNode<M>) -> Self {
		Self(Tree::new(item.into_behavior_node()))
	}

	pub fn child<M>(mut self, child: impl IntoBehaviorTree<M>) -> Self {
		self.0 = self.0.with_child(child.into_behavior_tree().0);
		self
	}

	pub fn spawn(
		&self,
		world: &mut impl WorldOrCommands,
		agent: Entity,
	) -> EntityGraph {
		EntityGraph::spawn(world, self.clone(), agent)
	}

	pub fn into_behavior_graph(self) -> BehaviorGraph {
		BehaviorGraph(self.0.into_graph())
	}
}

// impl Into<BehaviorGraph> for &BehaviorTree {
// 	fn into(self) -> BehaviorGraph { self.clone().into_behavior_graph() }
// }
// impl Into<BehaviorGraph> for BehaviorTree {
// 	fn into(self) -> BehaviorGraph { self.into_behavior_graph() }
// }

pub trait IntoBehaviorTree<M> {
	fn into_behavior_tree(self) -> BehaviorTree;
}

pub struct IntoIntoBehaviorTree;
pub struct NodeIntoBehaviorTree;

impl<T> IntoBehaviorTree<IntoIntoBehaviorTree> for T
where
	T: Into<BehaviorTree>,
{
	fn into_behavior_tree(self) -> BehaviorTree { self.into() }
}

impl<M, T> IntoBehaviorTree<(M, NodeIntoBehaviorTree)> for T
where
	T: IntoBehaviorNode<M>,
{
	fn into_behavior_tree(self) -> BehaviorTree { BehaviorTree::new(self) }
}
