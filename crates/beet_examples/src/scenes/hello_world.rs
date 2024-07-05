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
			parent.spawn((
				Name::new("World 1"),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
			parent.spawn((
				Name::new("World 2"),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
			parent.spawn((
				Name::new("World 3"),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
			parent.spawn((
				Name::new("World 4"),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
		});
}
