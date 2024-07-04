use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn hello_net(mut commands: Commands) {
	commands
		.spawn((SequenceSelector::default(), Running))
		.with_children(|parent| {
			parent.spawn((
				Name::new("Send - AppReady"),
				TriggerOnRun(AppReady),
			));
		});
	commands.spawn((
		Name::new("Recv - Player Message"),
		InsertOnTrigger::<OnUserMessage, Running>::new(Running),
	));
}
