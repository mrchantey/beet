use beet::action_list;
use beet::prelude::*;


// for now we need to manually keep in sync with crates/beet_ecs/src/builtin_nodes/builtin_nodes.rs

action_list!(BeeNode, [
	//core
	Translate,
	//ecs
	EmptyAction,
	SetRunResult,
	SetScore,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
]);
