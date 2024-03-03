use beet_ecs::action_list;
use beet_ecs::prelude::*;

action_list!(MyNodes, [
	EmptyAction,
	SetRunResult,
	SetScore,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
]);

pub fn main() {}
