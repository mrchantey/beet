use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

pub fn hello_net(mut commands: Commands) {
	commands
		.spawn((SequenceSelector::default(), Running))
		.with_children(|parent| {
			parent.spawn((
				LogOnRun::new("Send: AppLoaded"),
				TriggerOnRun(AppLoaded),
			));
		});
	commands.spawn((
		InsertOnTrigger::<OnUserMessage, Running>::new(Running),
		LogOnRun::new("Recv: Player Message"),
	));
}
