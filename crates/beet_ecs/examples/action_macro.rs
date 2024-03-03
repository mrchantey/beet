use beet_ecs::prelude::*;



pub fn main() {}

#[action(system=foo)]
#[derive(Default)]
pub struct Foo {
	#[number(min = 0, max = 100, step = 1)]
	health: u32,
	#[shared]
	score: Score,
}

fn foo() {}
