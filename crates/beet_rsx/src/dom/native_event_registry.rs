use crate::prelude::*;
pub struct EventRegistry;



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
		loc: DomLocation,
		value: impl Fn(MockEvent) -> (),
	) {
		todo!()
	}
}
