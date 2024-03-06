use beet_ecs::prelude::*;
extern crate beet_ecs as beet;



pub fn main() {}

#[action(system=foo)]
pub struct Foo {
	score: Score,
}

fn foo() {}
