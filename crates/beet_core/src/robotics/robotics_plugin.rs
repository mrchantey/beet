use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;


pub struct RoboticsPlugin;

impl Plugin for RoboticsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<(
			SetAgentOnRun<DualMotorValue>,
			DepthSensorScorer,
		)>::default());

		let world = app.world_mut();
		world.init_bundle::<DepthValue>();
		world.init_bundle::<DualMotorValue>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();
		registry.register::<DepthValue>();
		registry.register::<DualMotorValue>();
	}
}
