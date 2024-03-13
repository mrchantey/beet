// use bevy_ecs::prelude::*;
// use bevy_reflect::Reflect;
extern crate beet_ecs as beet;
// use beet_ecs_macros::*;


pub fn main() {}

#[derive(Default, bevy_ecs::prelude::Component, bevy_reflect::Reflect)]
struct Bar;

// #[derive(Default)]
#[beet_ecs::prelude::derive_action(set=PreTickSet)]
struct Foo {
	// #[number(min = 0, max = 100, step = 1)]
	health: u32,
}
fn foo() {}




#[derive(
	Debug,
	Clone,
	bevy_ecs::prelude::Component,
	bevy_reflect::Reflect,
	beet_ecs::prelude::Action,
)]
#[action(set=PreTickSet,child_components=[Bar])]
struct Bazz {
	health: u32,
}
fn bazz() {}
