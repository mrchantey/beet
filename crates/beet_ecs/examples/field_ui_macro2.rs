use beet_ecs::prelude::*;
use beet_ecs_macros::FieldUi;
// use beet_ecs::prelude::*;
use strum_macros::Display;
use strum_macros::EnumIter;
extern crate beet_ecs as beet;

fn main() {}

#[derive(Clone, EnumIter, Display, FieldUi)]
// #[hide_ui]
pub enum AttackType {
	Foo,
	Bar { value: u32 },
	Baz,
}
