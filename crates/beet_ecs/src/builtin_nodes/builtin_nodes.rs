use crate::prelude::*;
use beet_ecs::action_list;

action_list!(BuiltinNode, [
	EmptyAction,
	SetRunResult,
	SetScore,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
]);
