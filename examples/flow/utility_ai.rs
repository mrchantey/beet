//! This example demonstrates utility ai with constant score providers,
//! see `malenia.rs` for custom score providers
//!
use beet::prelude::*;
use sweet::prelude::EntityWorldMutwExt;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((
			BeetFlowPlugin::default(),
			BeetDebugPlugin::default()
		))
		.world_mut()
		.spawn((
			Name::new("ScoreFlow will select the highest score"), 
			HighestScore::default(),
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("this child does not run"),
				ReturnWith(ScoreValue(0.4)),
			));
			parent.spawn((
				Name::new("this child runs"),
				ReturnWith(ScoreValue(0.6)),
			));
		})
		.trigger_entity(RUN).flush();
}
