use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Event, Reflect)]
pub struct OnChildValue<T> {
	child: Entity,
	value: T,
}

impl<T> OnChildValue<T> {
	pub fn new(child: Entity, value: T) -> Self { Self { child, value } }
	pub fn value(&self) -> &T { &self.value }
	pub fn child(&self) -> Entity { self.child }
}


impl<T: PartialEq> PartialEq for OnChildValue<T> {
	fn eq(&self, other: &Self) -> bool {
		self.child == other.child && self.value == other.value
	}
}
