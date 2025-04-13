use crate::prelude::*;
pub struct EventRegistry;


pub mod event {
	use super::*;
	pub type MouseEvent = MockEvent;
}

pub struct MockEvent {
	pub target: MockTarget,
}
pub struct MockTarget {
	pub value: String,
}


impl EventRegistry {
	#[allow(unused_variables)]
	pub fn register_onclick(
		key: &str,
		loc: TreeLocation,
		value: impl EventHandler<event::MouseEvent>,
	) {
		todo!()
	}
}
