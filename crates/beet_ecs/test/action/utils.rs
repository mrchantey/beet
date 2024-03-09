use beet_ecs::prelude::*;

///
/// Root
/// 	Child0
/// 	Child1
/// 		Child0
///

pub fn test_action_graph_typed() -> BehaviorGraph<EcsNode> {
	BehaviorTree::<EcsNode>::new(ConstantScore::default())
		.with_child(ConstantScore::default())
		.with_child(
			BehaviorTree::new(ConstantScore::default())
				.with_child(ConstantScore::default()),
		)
		.into_behavior_graph()
}
