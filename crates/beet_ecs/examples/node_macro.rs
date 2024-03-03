use beet_ecs::prelude::*;



pub fn main() {}

#[action(system=foo)]
pub struct Foo {
	score: Score,
}

fn foo() {}
