#![allow(unused)]
// use beet_ecs::action_list;
use beet_ecs_macros::ActionList;
// use beet_ecs::exports::Display;
// use beet_ecs::prelude::*;
use bevy::prelude::Transform;

extern crate beet_ecs as beet;

// struct Foobar;

// SetOnStart::<Score>,
#[derive(ActionList)]
#[actions(
	SetOnStart::<Score>,
	EmptyAction,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	ScoreSelector
)]
#[components(Transform)]
struct MyNodes;




pub fn main() {}
