use beet_spatial::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// For apps and scenes that use beet_spatial
pub fn beet_spatial_scene_plugin(app: &mut App) {
	app
		.add_plugins(ActionPlugin::<(
			RemoveOnTrigger<OnRunResult, SteerTarget>,
			RemoveOnTrigger<OnRunResult, Velocity>,
			InsertOnTrigger<OnRun, Velocity>,
			RemoveOnTrigger<OnRun, Velocity>,
		)>::default())
		/*-*/;
}
