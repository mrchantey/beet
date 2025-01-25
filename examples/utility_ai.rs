//! This example demonstrates utility ai with constant score providers,
//! see `malenia.rs` for custom score providers
//!
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
