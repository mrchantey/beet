use crate::prelude::*;
use bevy_ecs::entity::Entity;


#[derive(Clone)]
pub struct WillyBehaviorTree(pub Tree<WillyBehaviorNode>);



impl WillyBehaviorTree {
	pub fn new<M>(item: impl IntoWillyBehaviorNode<M>) -> Self {
		Self(Tree::new(item.into_behavior_node()))
	}

	pub fn child<M>(mut self, child: impl IntoWillyBehaviorTree<M>) -> Self {
		self.0 = self.0.with_child(child.into_behavior_tree().0);
		self
	}

	pub fn spawn(
		&self,
		world: &mut impl WorldOrCommands,
		target: Entity,
	) -> EntityGraph {
		EntityGraph::spawn(world, self, target)
	}

	pub fn into_behavior_graph(self) -> WillyBehaviorGraph {
		WillyBehaviorGraph(self.0.into_graph())
	}
}

impl Into<WillyBehaviorGraph> for &WillyBehaviorTree {
	fn into(self) -> WillyBehaviorGraph { self.clone().into_behavior_graph() }
}
impl Into<WillyBehaviorGraph> for WillyBehaviorTree {
	fn into(self) -> WillyBehaviorGraph { self.into_behavior_graph() }
}

pub trait IntoWillyBehaviorTree<M> {
	fn into_behavior_tree(self) -> WillyBehaviorTree;
}

impl<M, T> IntoWillyBehaviorTree<(M, NodeIntoWillyBehaviorTree)> for T
where
	T: IntoWillyBehaviorNode<M>,
{
	fn into_behavior_tree(self) -> WillyBehaviorTree {
		WillyBehaviorTree::new(self)
	}
}

pub struct NodeIntoWillyBehaviorTree;
pub struct IntoIntoWillyBehaviorTree;

impl<T> IntoWillyBehaviorTree<IntoIntoWillyBehaviorTree> for T
where
	T: Into<WillyBehaviorTree>,
{
	fn into_behavior_tree(self) -> WillyBehaviorTree { self.into() }
}


// impl<M, T> Into<WillyBehavoirGraph> for T
// where
// 	T: IntoWillyBehaviorNode<M>,
// {
// 	fn into(self) -> WillyBehavoirGraph {
// 		WillyBehaviorTree::new(self)
// 	}
// }
