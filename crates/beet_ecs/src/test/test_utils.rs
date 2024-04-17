use crate::prelude::*;
pub fn test_constant_behavior_tree() -> BeetBuilder {
	(Score::default(), SetOnSpawn::<Score>::default())
		.child((Score::default(), SetOnSpawn::<Score>::default()))
		.child(
			(Score::default(), SetOnSpawn::<Score>::default())
				.child((Score::default(), SetOnSpawn::<Score>::default())),
		)
}

pub fn test_no_action_behavior_tree() -> BeetBuilder {
	EmptyAction
		.child(EmptyAction)
		.child(EmptyAction.child(EmptyAction))
}



pub fn test_serde_tree() -> BeetBuilder {
	BeetBuilder::new((
		SetOnSpawn::<Score>::default(),
		InsertOnRun::<RunResult>::default(),
		// utility
		EmptyAction::default(),
		Repeat::default(),
		InsertInDuration::<RunResult>::default(),
		// selectors
		SequenceSelector::default(),
		FallbackSelector::default(),
		ScoreSelector::default(),
	))
}
