use crate::prelude::*;
use bevy::prelude::*;

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