// use beet_ecs_macros::InspectorOptions;
extern crate beet_ecs as beet;
use beet_ecs::prelude::InspectorOptions;
fn main() {}

#[derive(InspectorOptions)]
struct MyStruct {
	#[inspector(min = 0.1, step = 3., max = 3.)]
	_field: f32,
	// #[inspector(hidden)]
	// _field2:bool
}
