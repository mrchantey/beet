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
			Name::new("ScoreFlow will select the highest score"), 
			ScoreFlow::default(),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("this child does not run"),
				ScoreProvider::new(0.4),
			));
			parent.spawn((
				Name::new("this child runs"),
				ScoreProvider::new(0.6),
			));
		})
		.flush_trigger(OnRun);
}
