#![allow(unused)]
// use beet_ecs::action_list;
use beet_ecs_macros::ActionList;
// use beet_ecs::exports::Display;
// use beet_ecs::prelude::*;

extern crate beet_ecs as beet;

// SetOnStart::<Score>,
#[derive(ActionList)]
#[actions(
	SetOnStart::<Score>,
	EmptyAction,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
)]
struct MyNodes;




pub fn main() {}
