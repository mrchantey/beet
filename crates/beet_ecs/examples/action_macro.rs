use beet_ecs::prelude::*;
extern crate beet_ecs as beet;



pub fn main() {}

#[action(system=foo)]
#[derive(Default)]
pub struct Foo {
	#[number(min = 0, max = 100, step = 1)]
	health: u32,
}

fn foo() {}
