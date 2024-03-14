// use bevy_ecs::prelude::*;
// use bevy_reflect::Reflect;
extern crate beet_ecs as beet;
// use beet_ecs_macros::*;
// use bevy_reflect::FromReflect;
// use bevy_reflect::GetTypeRegistration;
// use bevy_reflect::TypePath;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use std::fmt::Debug;

pub fn main() {}

// #[derive(Default, bevy_ecs::prelude::Component, bevy_reflect::Reflect)]
// struct Bar;

// // #[derive(Default)]
// #[beet_ecs::prelude::derive_action(set=PreTickSet)]
// struct Foo {
// 	// #[number(min = 0, max = 100, step = 1)]
// 	health: u32,
// }
// fn foo() {}

#[beet_ecs::prelude::derive_action(set=PreTickSet)]
struct Action1<T: 'static + Component>
where
	T: Debug,
{
	// #[number(min = 0, max = 100, step = 1)]
	health: T,
}
fn action1<T: Component>() {}


#[derive(PartialEq, Deref, DerefMut)]
#[beet_ecs::prelude::derive_action(set=PreTickSet)]
struct Action2<T: 'static + Component>(pub T);

fn action2<T: Component>() {}


// #[beet_ecs::prelude::derive_action(set=PreTickSet)]
// enum MyEnum<T>
// where
// 	T: Debug,
// {
// 	// #[number(min = 0, max = 100, step = 1)]
// 	A,
// 	B(T),
// }
// fn my_enum<T>() {}




// #[derive(
// 	Debug,
// 	Clone,
// bevy_ecs::prelude::Component,
// 	bevy_reflect::Reflect,
// 	beet_ecs::prelude::Action,
// )]
// #[action(set=PreTickSet,child_components=[Bar])]
// struct Bazz {
// 	health: u32,
// }
// fn bazz() {}
