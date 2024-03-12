use beet_ecs::prelude::*;

///
/// Root
/// 	Child0
/// 	Child1
/// 		Child0
///

pub fn test_constant_behavior_tree() -> BehaviorTree {
	ConstantScore::default()
		.child(ConstantScore::default())
		.child(ConstantScore::default().child(ConstantScore::default()))
}

pub fn test_no_action_behavior_tree() -> BehaviorTree {
	EmptyAction
		.child(EmptyAction)
		.child(EmptyAction.child(EmptyAction))
}
