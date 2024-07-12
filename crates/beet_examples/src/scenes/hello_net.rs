use crate::beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub fn hello_net(mut commands: Commands) {
	commands.spawn((
		Name::new("Recv - AppReady"), 
		RunOnAppReady::default()
	));
	commands.spawn((
		Name::new("Send - AppReady"),
		RunOnSpawn,
		TriggerOnRun::new(AppReady)
	));
}
