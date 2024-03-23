use beet_ecs_macros::InspectorOptions;
extern crate beet_ecs as beet;

fn main() {}


#[derive(InspectorOptions)]
struct MyStruct {
	#[inspector(min = 0.1, max = 3.)]
	_field: f32,
}
