// use beet_ecs_macros::InspectorOptions;
extern crate beet_ecs as beet;
use beet_ecs::prelude::InspectorOptions;
use beet_ecs_macros::derive_action;
fn main() {}


// #[derive(InspectorOptions)]
#[derive_action]
struct MyStruct {
	#[inspector(min = 0.1, step = 3., max = 3.)]
	_field: f32,
	// #[inspector(hidden)]
	// _field2:bool
}

fn my_struct() {}


// #[derive(Bundle)]
// struct FooBar {
// 	foo: bevy::prelude::Transform,
// 	bar: bevy::prelude::GlobalTransform,
// }

// #[derive(Component)]
// struct Fizz;
