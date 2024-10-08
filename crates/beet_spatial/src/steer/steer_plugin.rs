use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Required Resources:
/// - [`Time`]
#[derive(Default)]
pub struct SteerPlugin;


impl Plugin for SteerPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<(
			Seek,
			Wander,
			Separate<GroupSteerAgent>,
			Align<GroupSteerAgent>,
			Cohere<GroupSteerAgent>,
			EndOnArrive,
			FindSteerTarget,
			SteerTargetScoreProvider,
			DespawnSteerTarget,
			RunOnSteerTargetInsert,
			RunOnSteerTargetRemove,
		)>::default())
		.register_type::<SteerTarget>()
		.register_type::<MaxForce>()
		.register_type::<MaxSpeed>()
		.register_type::<ArriveRadius>()
		.register_type::<GroupSteerAgent>()
		/*_*/;

		let world = app.world_mut();
		world.register_bundle::<SteerBundle>();
	}
}

