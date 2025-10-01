//! Base actions suitable for robotics and robotics simulation.
//! These are implementation agnostic, for example an esp32 would
//! implement these actions differently than a simulation.
mod depth;
pub use depth::*;
mod depth_sensor_scorer;
pub use depth_sensor_scorer::*;
mod dual_motor;
pub use dual_motor::*;
mod motor;
use beet_flow::prelude::*;
use beet_core::prelude::*;
pub use motor::*;

/// A plugin that registers all robotics components and bundles:
pub struct RoboticsPlugin;

impl Plugin for RoboticsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<DepthValue>()
			.register_type::<DualMotorValue>();

		let world = app.world_mut();
		world.register_bundle::<DepthValue>();
		world.register_bundle::<DualMotorValue>();
	}
}
