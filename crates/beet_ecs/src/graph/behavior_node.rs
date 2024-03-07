use crate::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::ops::Deref;
use std::ops::DerefMut;

pub trait ActionSuper: Clone + PartialEq + Action {}
impl<T: Clone + PartialEq + Action> ActionSuper for T {}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorNode<T: Action> {
	pub name: String,
	pub actions: Vec<T>,
}


// impl<T: Action> Into<BehaviorNode<T>> for Vec<T> {
// 	fn into(self) -> BehaviorNode<T> {}
// }

pub const NEW_NAME: &str = "New Node";

impl<T: Action> Deref for BehaviorNode<T> {
	type Target = Vec<T>;
	fn deref(&self) -> &Self::Target { &self.actions }
}
impl<T: Action> DerefMut for BehaviorNode<T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.actions }
}
impl<T: Action> Default for BehaviorNode<T> {
	fn default() -> Self {
		Self {
			actions: Vec::new(),
			name: NEW_NAME.to_string(),
		}
	}
}

impl<T: Action> BehaviorNode<T> {
	pub fn empty() -> Self { Self::default() }
	pub fn new(actions: Vec<T>) -> Self {
		Self {
			name: NEW_NAME.to_string(),
			actions,
		}
	}
}


pub trait IntoBehaviorNode<M, T: Action> {
	fn into_behavior_node(self) -> BehaviorNode<T>;
}

pub struct ItemIntoBehaviorNode;
pub struct VecIntoBehaviorNode;
pub struct IntoIntoBehaviorNode;

impl<T: Action, U> IntoBehaviorNode<IntoIntoBehaviorNode, T> for U
where
	U: Into<BehaviorNode<T>>,
{
	fn into_behavior_node(self) -> BehaviorNode<T> { self.into() }
}

impl<T: Action> IntoBehaviorNode<VecIntoBehaviorNode, T> for Vec<T> {
	fn into_behavior_node(self) -> BehaviorNode<T> { BehaviorNode::new(self) }
}


impl<T: Action, U1: Into<T>> IntoBehaviorNode<(ItemIntoBehaviorNode, U1), T>
	for U1
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let u1 = self;
		BehaviorNode::new(vec![u1.into()])
	}
}
impl<T: Action, U1: Into<T>, U2: Into<T>>
	IntoBehaviorNode<(ItemIntoBehaviorNode, U1, U2), T> for (U1, U2)
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let (u1, u2) = self;
		BehaviorNode::new(vec![u1.into(), u2.into()])
	}
}
impl<T: Action, U1: Into<T>, U2: Into<T>, U3: Into<T>>
	IntoBehaviorNode<(ItemIntoBehaviorNode, U1, U2, U3), T> for (U1, U2, U3)
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let (u1, u2, u3) = self;
		BehaviorNode::new(vec![u1.into(), u2.into(), u3.into()])
	}
}

impl<T: Action, U1: Into<T>, U2: Into<T>, U3: Into<T>, U4: Into<T>>
	IntoBehaviorNode<(ItemIntoBehaviorNode, U1, U2, U3, U4), T>
	for (U1, U2, U3, U4)
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let (u1, u2, u3, u4) = self;
		BehaviorNode::new(vec![u1.into(), u2.into(), u3.into(), u4.into()])
	}
}

impl<
		T: Action,
		U1: Into<T>,
		U2: Into<T>,
		U3: Into<T>,
		U4: Into<T>,
		U5: Into<T>,
	> IntoBehaviorNode<(ItemIntoBehaviorNode, U1, U2, U3, U4, U5), T>
	for (U1, U2, U3, U4, U5)
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let (u1, u2, u3, u4, u5) = self;
		BehaviorNode::new(vec![
			u1.into(),
			u2.into(),
			u3.into(),
			u4.into(),
			u5.into(),
		])
	}
}

impl<
		T: Action,
		U1: Into<T>,
		U2: Into<T>,
		U3: Into<T>,
		U4: Into<T>,
		U5: Into<T>,
		U6: Into<T>,
	> IntoBehaviorNode<(ItemIntoBehaviorNode, U1, U2, U3, U4, U5, U6), T>
	for (U1, U2, U3, U4, U5, U6)
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let (u1, u2, u3, u4, u5, u6) = self;
		BehaviorNode::new(vec![
			u1.into(),
			u2.into(),
			u3.into(),
			u4.into(),
			u5.into(),
			u6.into(),
		])
	}
}

impl<
		T: Action,
		U1: Into<T>,
		U2: Into<T>,
		U3: Into<T>,
		U4: Into<T>,
		U5: Into<T>,
		U6: Into<T>,
		U7: Into<T>,
	> IntoBehaviorNode<(ItemIntoBehaviorNode, U1, U2, U3, U4, U5, U6, U7), T>
	for (U1, U2, U3, U4, U5, U6, U7)
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let (u1, u2, u3, u4, u5, u6, u7) = self;
		BehaviorNode::new(vec![
			u1.into(),
			u2.into(),
			u3.into(),
			u4.into(),
			u5.into(),
			u6.into(),
			u7.into(),
		])
	}
}

impl<
		T: Action,
		U1: Into<T>,
		U2: Into<T>,
		U3: Into<T>,
		U4: Into<T>,
		U5: Into<T>,
		U6: Into<T>,
		U7: Into<T>,
		U8: Into<T>,
	> IntoBehaviorNode<(ItemIntoBehaviorNode, U1, U2, U3, U4, U5, U6, U7, U8), T>
	for (U1, U2, U3, U4, U5, U6, U7, U8)
{
	fn into_behavior_node(self) -> BehaviorNode<T> {
		let (u1, u2, u3, u4, u5, u6, u7, u8) = self;
		BehaviorNode::new(vec![
			u1.into(),
			u2.into(),
			u3.into(),
			u4.into(),
			u5.into(),
			u6.into(),
			u7.into(),
			u8.into(),
		])
	}
}
