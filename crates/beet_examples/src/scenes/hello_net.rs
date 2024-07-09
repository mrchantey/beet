use crate::beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
pub fn hello_net(mut commands: Commands) {
	let target = commands.spawn((
		Name::new("Recv - AppReady"), 
		RunOnAppReady::default()
	)).id();
	commands.spawn((
		Name::new("Send - AppReady"),
		RunOnSpawn,
		TriggerOnRun::new(AppReady).with_target(target)
	));
}
