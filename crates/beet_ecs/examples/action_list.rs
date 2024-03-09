use beet_ecs::action_list;
// use beet_ecs::exports::Display;
// use beet_ecs::prelude::*;

extern crate beet_ecs as beet;

action_list!(MyNodes, [
	EmptyAction,
	SetRunResult,
	ConstantScore,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
]);

pub fn main() {}
