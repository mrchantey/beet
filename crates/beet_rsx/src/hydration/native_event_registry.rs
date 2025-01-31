use crate::prelude::*;
pub struct EventRegistry;



pub struct MockEvent {
	pub target: MockTarget,
}
pub struct MockTarget {
	pub value: String,
}


impl EventRegistry {
	pub fn register_onclick(
		_key: &str,
		_cx: &RsxContext,
		_value: impl Fn(MockEvent) -> (),
	) {
		todo!()
	}
}
