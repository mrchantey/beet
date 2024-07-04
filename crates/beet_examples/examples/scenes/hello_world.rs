use beet::prelude::*;
use bevy::prelude::*;

pub fn hello_world(mut commands: Commands) {
	commands
		.spawn((SequenceSelector::default(), Running))
		.with_children(|parent| {
			parent.spawn((
				LogOnRun("Hello".into()),
				InsertOnRun(RunResult::Success),
			));
			parent.spawn((
				LogOnRun("World".into()),
				InsertOnRun(RunResult::Success),
			));
			parent.spawn((
				LogOnRun("World 1".into()),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
			parent.spawn((
				LogOnRun("World 2".into()),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
			parent.spawn((
				LogOnRun("World 3".into()),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
			parent.spawn((
				LogOnRun("World 4".into()),
				RunTimer::default(),
				InsertInDuration::with_secs(RunResult::Success, 1),
			));
		});
}
