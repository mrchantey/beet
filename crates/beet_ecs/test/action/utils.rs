use beet_ecs::prelude::*;

///
/// Root
/// 	Child0
/// 	Child1
/// 		Child0
///
// pub fn test_action_graph_boxed() -> BoxedBehaviorGraph {
// 	BoxedBehaviorTree::new(vec![Box::new(SetScore::default())])
// 		.with_leaf(vec![Box::new(SetScore::default())])
// 		.with_child(
// 			BoxedBehaviorTree::new(vec![Box::new(SetScore::default())])
// 				.with_leaf(vec![Box::new(SetScore::default())]),
// 		)
// 		.into_action_graph()
// }

pub fn test_action_graph_typed() -> BehaviorGraph<BuiltinNode> {
	BehaviorTree::<BuiltinNode>::new(
		vec![BuiltinNode::SetScore(SetScore::default())].into(),
	)
	.with_leaf(vec![BuiltinNode::SetScore(SetScore::default())].into())
	.with_child(
		Tree::new(vec![BuiltinNode::SetScore(SetScore::default())].into())
			.with_leaf(vec![BuiltinNode::SetScore(SetScore::default())].into()),
	)
	.into_action_graph()
}
