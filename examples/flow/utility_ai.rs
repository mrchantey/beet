//! This example demonstrates utility ai with constant score providers,
//! see `malenia.rs` for custom score providers
//!
use beet::prelude::*;

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
			children![
				(
					Name::new("this child does not run"),
					EndOnRun(ScoreValue(0.4)),
				),
				(
					Name::new("this child runs"),
					EndOnRun(ScoreValue(0.6)),
				)
			]
		))
		.trigger_entity(RUN).flush();
}
