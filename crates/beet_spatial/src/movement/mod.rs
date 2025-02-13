pub mod force_bundle;
#[allow(unused_imports)]
pub use self::force_bundle::*;
pub mod hover;
#[allow(unused_imports)]
pub use self::hover::*;
pub mod integrate_force;
#[allow(unused_imports)]
pub use self::integrate_force::*;
pub mod movement_plugin;
#[allow(unused_imports)]
pub use self::movement_plugin::*;
pub mod rotate_to_velocity;
#[allow(unused_imports)]
pub use self::rotate_to_velocity::*;
pub mod translate;
#[allow(unused_imports)]
pub use self::translate::*;

use super::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<(
			Hover,
			Translate,
		)>::default()).add_systems(
			Update,
			(
				integrate_force,
				hover.in_set(TickSet),
				(rotate_to_velocity_2d, rotate_to_velocity_3d),
			)
				.chain()
				.in_set(PostTickSet),
		)		.register_type::<Mass>()
		.register_type::<Velocity>()
		.register_type::<Impulse>()
		.register_type::<Force>()
		.register_type::<RotateToVelocity2d>()
		.register_type::<RotateToVelocity3d>()
		.register_type::<VelocityScalar>()		
		/*-*/;

		let world = app.world_mut();
		world.register_bundle::<ForceBundle>();
	}
}
