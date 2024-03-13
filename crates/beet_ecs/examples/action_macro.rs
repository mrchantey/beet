use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
extern crate beet_ecs as beet;
// use beet_ecs_macros::*;


pub fn main() {}

#[derive(Default, Component, Reflect)]
struct Bar;

// #[derive(Default)]
#[derive(Debug, Clone, Component, Reflect, Action)]
#[reflect(Component, Action)]
#[action(set=PreTickSet,child_components=[Bar])]
struct Foo {
	// #[number(min = 0, max = 100, step = 1)]
	health: u32,
}

fn foo() {}
