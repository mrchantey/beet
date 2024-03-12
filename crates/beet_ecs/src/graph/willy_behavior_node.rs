use crate::prelude::*;
use bevy_ecs::all_tuples;


pub struct WillyBehaviorNode {
	pub name: String,
	pub actions: Vec<Box<dyn Action>>,
}

impl Clone for WillyBehaviorNode {
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

impl<T: Action> Into<WillyBehaviorNode> for &BehaviorNode<T> {
	fn into(self) -> WillyBehaviorNode {
		WillyBehaviorNode {
			name: self.name.clone(),
			actions: self
				.actions
				.iter()
				.map(|action| action.duplicate() as Box<dyn Action>)
				.collect(),
		}
	}
}

pub struct ItemIntoWillyBehaviorNode;
pub struct TupleIntoWillyBehaviorNode;

pub trait IntoWillyBehaviorNode<M>: Sized {
	fn into_behavior_node(self) -> WillyBehaviorNode;


	fn child<M2>(
		self,
		child: impl IntoWillyBehaviorTree<M2>,
	) -> WillyBehaviorTree {
		WillyBehaviorTree::new(self).child(child)
	}
}
impl<T0: Action> IntoWillyBehaviorNode<ItemIntoWillyBehaviorNode> for T0 {
	#[allow(unused_variables, unused_mut)]
	fn into_behavior_node(self) -> WillyBehaviorNode {
		#[allow(non_snake_case)]
		let T0 = self;
		WillyBehaviorNode {
			name: NEW_NAME.to_string(),
			actions: vec![T0.duplicate()],
		}
	}
}

macro_rules! tuple_into_behavior_node {
	($($T:ident),*) => {
			impl<$($T:Action),*> IntoWillyBehaviorNode<TupleIntoWillyBehaviorNode> for ($($T,)*) {
				#[allow(unused_variables, unused_mut)]
				fn into_behavior_node(self) -> WillyBehaviorNode {
					#[allow(non_snake_case)]
					let ($($T,)*) = self;
					WillyBehaviorNode {
						name: NEW_NAME.to_string(),
						actions: vec![$($T.duplicate(),)*],
					}
				}
			}
	};
}

all_tuples!(tuple_into_behavior_node, 0, 15, T);
