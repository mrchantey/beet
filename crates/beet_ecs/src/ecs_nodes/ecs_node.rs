use crate::action_list;

action_list!(EcsNode, [
	ConstantScore,
	EmptyAction,
	FallbackSelector,
	Repeat,
	SetRunResult,
	SequenceSelector,
	SucceedInDuration,
	UtilitySelector
]);
