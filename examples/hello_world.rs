//! A basic behavior tree sequence example
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins(BeetFlowPlugin::default().log_on_run())
		.world_mut()
		.spawn((
			Name::new("root"), 
			SequenceFlow
		))
		.with_child((
			Name::new("child1"),
			EndOnRun::success(),
		))
		.with_child((
			Name::new("child2"),
			EndOnRun::success(),
		))
		.flush_trigger(OnRun);
}
