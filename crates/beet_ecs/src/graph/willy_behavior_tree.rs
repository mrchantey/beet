use crate::prelude::*;
use bevy_ecs::entity::Entity;


#[derive(Clone)]
pub struct WillyBehaviorTree(pub Tree<WillyBehaviorNode>);



impl WillyBehaviorTree {
	pub fn new<M>(item: impl IntoWillyBehaviorNode<M>) -> Self {
		Self(Tree::new(item.into_behavior_node()))
	}

	pub fn with_child<M>(
		mut self,
		child: impl IntoWillyBehaviorTree<M>,
	) -> Self {
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

	pub fn into_behavior_graph(self) -> WillyBehavoirGraph {
		WillyBehavoirGraph(self.0.into_graph())
	}
}

impl Into<WillyBehavoirGraph> for &WillyBehaviorTree {
	fn into(self) -> WillyBehavoirGraph { self.clone().into_behavior_graph() }
}
impl Into<WillyBehavoirGraph> for WillyBehaviorTree {
	fn into(self) -> WillyBehavoirGraph { self.into_behavior_graph() }
}

pub trait IntoWillyBehaviorTree<M> {
	fn into_behavior_tree(self) -> WillyBehaviorTree;
}


impl<M, T> IntoWillyBehaviorTree<M> for T
where
	T: IntoWillyBehaviorNode<M>,
{
	fn into_behavior_tree(self) -> WillyBehaviorTree {
		WillyBehaviorTree::new(self)
	}
}
