use beet::prelude::*;
use bevy::prelude::*;

pub fn hello_world(mut commands: Commands) {
	commands
		.spawn((SequenceSelector::default(), Running))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Hello"),
				InsertOnRun(RunResult::Success),
			));
			parent.spawn((
				Name::new("World"),
				InsertOnRun(RunResult::Success),
			));
		});
}
