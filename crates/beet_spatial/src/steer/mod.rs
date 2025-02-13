pub mod algo;
#[cfg(feature = "render")]
pub mod debug_group_steer;
#[cfg(feature = "render")]
pub use self::debug_group_steer::*;
pub mod steer_actions;
pub mod steer_bundle;
pub use self::steer_bundle::*;
pub mod steer_target;
pub use self::steer_target::*;
use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use sweet::prelude::RandomSource;

type M = GroupSteerAgent;

/// Required Resources:
/// - [`Time`]
pub fn steer_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(
			find_steer_target,
			end_on_arrive,
			seek,
			wander,
			separate::<M>,
			align::<M>,
			cohere::<M>,
		)
			.in_set(TickSet),
	)
	.init_resource::<RandomSource>()
	.register_type::<SteerTarget>()
	.register_type::<MaxForce>()
	.register_type::<MaxSpeed>()
	.register_type::<ArriveRadius>()
	.register_type::<GroupSteerAgent>();

	let world = app.world_mut();
	world.register_bundle::<SteerBundle>();
}
