use crate::prelude::*;
pub fn test_constant_behavior_tree() -> BeetNode {
	(Score::default(), SetOnStart::<Score>::default())
		.child((Score::default(), SetOnStart::<Score>::default()))
		.child(
			(Score::default(), SetOnStart::<Score>::default())
				.child((Score::default(), SetOnStart::<Score>::default())),
		)
}

pub fn test_no_action_behavior_tree() -> BeetNode {
	EmptyAction
		.child(EmptyAction)
		.child(EmptyAction.child(EmptyAction))
}
