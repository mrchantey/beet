use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


pub struct RoboticsPlugin;

impl Plugin for RoboticsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<(
			InsertOnTrigger<OnRun, DualMotorValue>,
			DepthSensorScorer,
		)>::default())
			.register_type::<DepthValue>()
			.register_type::<DualMotorValue>();

		let world = app.world_mut();
		world.init_bundle::<DepthValue>();
		world.init_bundle::<DualMotorValue>();
	}
}
