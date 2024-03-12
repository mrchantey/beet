use crate::prelude::*;
use bevy_ecs::all_tuples;

pub const NEW_NAME: &str = "New Node";

#[derive(Debug)]
pub struct BehaviorNode {
	pub name: String,
	pub actions: Vec<Box<dyn Action>>,
}
impl Default for BehaviorNode {
	fn default() -> Self {
		Self {
			name: NEW_NAME.to_string(),
			actions: Vec::new(),
		}
	}
}
impl BehaviorNode {
	pub fn new(actions: Vec<Box<dyn Action>>) -> Self {
		Self {
			name: NEW_NAME.to_string(),
			actions,
		}
	}
	pub fn named<M>(
		name: impl Into<String>,
		actions: impl IntoBehaviorNode<M>,
	) -> Self {
		let mut this = actions.into_behavior_node();
		this.name = name.into();
		this
	}
}

impl Clone for BehaviorNode {
	fn clone(&self) -> Self {
		Self {
			name: self.name.clone(),
			actions: self
				.actions
				.iter()
				.map(|action| action.duplicate())
				.collect(),
		}
	}
}

pub struct ItemIntoBehaviorNode;
pub struct TupleIntoBehaviorNode;

pub trait IntoBehaviorNode<M>: Sized {
	fn into_behavior_node(self) -> BehaviorNode;

	fn child<M2>(self, child: impl IntoBehaviorTree<M2>) -> BehaviorTree {
		BehaviorTree::new(self).child(child)
	}
}
impl<T0: Action> IntoBehaviorNode<ItemIntoBehaviorNode> for T0 {
	#[allow(unused_variables, unused_mut)]
	fn into_behavior_node(self) -> BehaviorNode {
		#[allow(non_snake_case)]
		let T0 = self;
		BehaviorNode::new(vec![T0.duplicate()])
	}
}

macro_rules! tuple_into_behavior_node {
	($($T:ident),*) => {
			impl<$($T:Action),*> IntoBehaviorNode<TupleIntoBehaviorNode> for ($($T,)*) {
				#[allow(unused_variables, unused_mut)]
				fn into_behavior_node(self) -> BehaviorNode {
					#[allow(non_snake_case)]
					let ($($T,)*) = self;
					BehaviorNode::new(vec![$($T.duplicate(),)*])
				}
			}
	};
}

all_tuples!(tuple_into_behavior_node, 0, 15, T);
