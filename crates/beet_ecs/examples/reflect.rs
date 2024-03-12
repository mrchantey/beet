use bevy_reflect::prelude::*;

pub fn main() {}

#[derive(Reflect)]
pub struct SomeValue(pub i32);

#[derive(Reflect)]
struct MyAction {
	pub val: SomeValue,
}
