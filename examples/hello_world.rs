use beet::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
    .insert_resource(BeetDebugConfig::default())
		.add_plugins((
			LogPlugin::default(), 
			BeetDebugPluginBase,
			BeetDebugPluginStdout,
			LifecyclePlugin,
		))
		.world_mut()
		.spawn((
			Name::new("root"), 
			SequenceFlow
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("child1"),
				EndOnRun::success(),
			));
			parent.spawn((
				Name::new("child2"),
				EndOnRun::success(),
			));
		})
		.flush_trigger(OnRun);
}
