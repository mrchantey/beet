//! A basic behavior tree sequence example
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
    .insert_resource(BeetDebugConfig::default())
		.add_plugins((
			LifecyclePlugin,
			BeetDebugPlugin,
			bevy::log::LogPlugin::default()
		))
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
