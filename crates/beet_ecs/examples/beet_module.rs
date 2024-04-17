#![allow(unused)]
// use beet_ecs::action_list;
use beet_ecs_macros::BeetModule;
// use beet_ecs::exports::Display;
// use beet_ecs::prelude::*;
use bevy::prelude::Transform;

extern crate beet_ecs as beet;

// struct Foobar;

// SetOnSpawn::<Score>,
#[derive(BeetModule)]
#[actions(
	SetOnSpawn::<Score>,
	EmptyAction,
	InsertInDuration::<RunResult>,
	SequenceSelector,
	FallbackSelector,
	ScoreSelector
)]
#[components(Transform)]
// #[bundles(TransformBundle)]
struct MyNodes;




pub fn main() {}
