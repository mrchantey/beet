use beet::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((
			LogPlugin::default(), 
			BeetObserverPlugin
		))
		.world_mut()
		.spawn((
			Name::new("root"), 
			LogNameOnRun, 
			SequenceFlow
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("child1"),
				LogNameOnRun,
				EndOnRun::success(),
			));
			parent.spawn((
				Name::new("child2"),
				LogNameOnRun,
				EndOnRun::success(),
			));
		})
		// trigger OnRun for the root
		.flush_trigger(OnRun);
}
