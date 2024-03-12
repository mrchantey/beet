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


pub fn set_run_result_graph() -> BehaviorGraph {
	BehaviorTree::new(SetRunResult::success()).into_behavior_graph()
}
