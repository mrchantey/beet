use beet::prelude::*;
use bevy::prelude::*;


pub fn hello_world(mut commands: Commands) {
	commands
		.spawn((
			Name::new("Hello World Sequence"),
			RunOnSpawn,
			SequenceFlow::default(),
		))
		.with_children(|parent| {
			parent.spawn((Name::new("Hello"), EndOnRun::success()));
			parent.spawn((Name::new("World"), EndOnRun::success()));
		});
}
