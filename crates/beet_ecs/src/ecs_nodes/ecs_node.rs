use crate::action_list;

action_list!(EcsNode, [
	EmptyAction,
	SetRunResult,
	SetScore,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
]);


pub fn set_run_result_graph() -> BehaviorGraph<EcsNode> {
	BehaviorTree::new(SetRunResult::success()).into_behavior_graph()
}
