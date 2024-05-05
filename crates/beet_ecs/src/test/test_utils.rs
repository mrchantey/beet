use crate::prelude::*;
use bevy::prelude::*;

pub fn test_constant_behavior_tree(world: &mut World) -> EntityTree {
	let entity = world
		.spawn((Running, Score::default(), SetOnSpawn::<Score>::default()))
		.with_children(|parent| {
			parent.spawn((Score::default(), SetOnSpawn::<Score>::default()));
			parent
				.spawn((Score::default(), SetOnSpawn::<Score>::default()))
				.with_children(|parent| {
					parent.spawn((
						Score::default(),
						SetOnSpawn::<Score>::default(),
					));
				});
		})
		.id();
	EntityTree::new_with_world(entity, world)
}

pub fn test_no_action_behavior_tree(world: &mut World) -> EntityTree {
	let entity = world
		.spawn(Running)
		.with_children(|parent| {
			parent.spawn_empty();
			parent.spawn_empty().with_children(|parent| {
				parent.spawn_empty();
			});
		})
		.id();
	EntityTree::new_with_world(entity, world)
}



pub fn test_serde_entity(world: &mut World) -> Entity {
	world
		.spawn((
			Running,
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
		.id()
}
