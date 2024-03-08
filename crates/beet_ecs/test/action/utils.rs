use beet_ecs::prelude::*;

///
/// Root
/// 	Child0
/// 	Child1
/// 		Child0
///

pub fn test_action_graph_typed() -> BehaviorGraph<EcsNode> {
	BehaviorTree::<EcsNode>::new(SetScore::default())
		.with_child(SetScore::default())
		.with_child(
			BehaviorTree::new(SetScore::default())
				.with_child(SetScore::default()),
		)
		.into_action_graph()
}
