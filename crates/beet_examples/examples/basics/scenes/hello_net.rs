use beet_examples::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;

pub fn hello_net(mut commands: Commands) {
	commands
		.spawn((SequenceSelector::default(), Running))
		.with_children(|parent| {
			parent.spawn((
				LogOnRun("Message Sent: AppLoaded".into()),
				TriggerOnRun(AppLoaded),
			));
		});
}
