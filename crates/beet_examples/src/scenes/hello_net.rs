use crate::beet::prelude::*;
use crate::prelude::*;
use bevy::prelude::*;

pub fn hello_net(mut commands: Commands) {
	commands
		.spawn((
			Name::new("Hello Net Sequence"),
			SequenceSelector::default(),
			Running
			// SequenceFlow::default(),
			// RunOnSpawn,
		))
		.with_children(|parent| {
			parent.spawn((Name::new("Send - AppReady"), SendOnRun(AppReady)));
		});
	commands.spawn((
		Name::new("Recv - OnUserMessage"),
		InsertOnTrigger::<OnUserMessage, Running>::new(Running),
	));
}
