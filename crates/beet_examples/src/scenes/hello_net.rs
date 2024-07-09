use crate::beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;

pub fn hello_net(mut commands: Commands) {
	commands
		.spawn((SequenceSelector::default(), Running))
		.with_children(|parent| {
			parent.spawn((Name::new("Send - AppReady"), SendOnRun(AppReady)));
		});
	commands.spawn((
		Name::new("Recv - Player Message"),
		InsertOnTrigger::<OnUserMessage, Running>::new(Running),
	));
}
