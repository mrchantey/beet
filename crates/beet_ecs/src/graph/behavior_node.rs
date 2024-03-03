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

impl<T: Action> Into<BehaviorNode<T>> for Vec<T> {
	fn into(self) -> BehaviorNode<T> {
		BehaviorNode {
			actions: self,
			name: "New Node".to_string(),
		}
	}
}


impl<T: Action> Deref for BehaviorNode<T> {
	type Target = Vec<T>;
	fn deref(&self) -> &Self::Target { &self.actions }
}
impl<T: Action> DerefMut for BehaviorNode<T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.actions }
}
impl<T: Action> Default for BehaviorNode<T> {
	fn default() -> Self { Vec::new().into() }
}

impl<T: Action> BehaviorNode<T> {
	pub fn new(actions: Vec<T>) -> Self { actions.into() }
}
