use crate::prelude::*;
use serde::Deserialize;
use serde::Serialize;



#[derive(Debug, Serialize, Deserialize)]
pub struct BehaviorTree<T: ActionSuper>(pub Tree<BehaviorNode<T>>);

impl<T: ActionSuper> Default for BehaviorTree<T> {
	fn default() -> Self { Self(Tree::new(BehaviorNode::empty())) }
}

impl<T: ActionSuper> BehaviorTree<T> {
	pub fn new<M>(item: impl IntoBehaviorNode<M, T>) -> Self {
		Self(Tree::new(item.into_behavior_node()))
	}

	pub fn with_child<M>(mut self, child: impl IntoBehaviorTree<M, T>) -> Self {
		self.0 = self.0.with_child(child.into_behavior_tree().0);
		self
	}

	pub fn into_action_graph(self) -> BehaviorGraph<T> {
		BehaviorGraph(self.0.into_graph())
	}
}

impl<T: ActionSuper> Into<BehaviorGraph<T>> for BehaviorTree<T> {
	fn into(self) -> BehaviorGraph<T> { self.into_action_graph() }
}
impl<T: ActionSuper> Into<Tree<BehaviorNode<T>>> for BehaviorTree<T> {
	fn into(self) -> Tree<BehaviorNode<T>> { self.0 }
}


pub trait IntoBehaviorTree<M, T: ActionSuper> {
	fn into_behavior_tree(self) -> BehaviorTree<T>;
}

pub struct IntoIntoBehaviorTree;
pub struct TreeIntoBehaviorTree;
pub struct NodeIntoBehaviorTree;

impl<T: ActionSuper, U> IntoBehaviorTree<IntoIntoBehaviorTree, T> for U
where
	U: Into<BehaviorTree<T>>,
{
	fn into_behavior_tree(self) -> BehaviorTree<T> { self.into() }
}
impl<T: ActionSuper> IntoBehaviorTree<TreeIntoBehaviorTree, T>
	for Tree<BehaviorNode<T>>
{
	fn into_behavior_tree(self) -> BehaviorTree<T> { BehaviorTree(self) }
}
impl<T: ActionSuper, U, M> IntoBehaviorTree<(NodeIntoBehaviorTree, M), T> for U
where
	U: IntoBehaviorNode<M, T>,
{
	fn into_behavior_tree(self) -> BehaviorTree<T> { BehaviorTree::new(self) }
}
