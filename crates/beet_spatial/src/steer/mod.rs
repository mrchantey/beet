//! Base systems and components for [steer actions](crate::steer_actions).
//! These are mostly based on the concepts of boids
//! from [Craig Reynolds](https://www.red3d.com/cwr/steer/),
//! with implementations derived from Daniel Shiffman's
//! [The Nature of Code](https://natureofcode.com/).
mod algo;
#[cfg(feature = "bevy_default")]
mod debug_group_steer;
#[cfg(feature = "bevy_default")]
pub use self::debug_group_steer::*;
mod steer_bundle;
pub use steer_bundle::*;
mod steer_target;
use crate::prelude::*;
pub use algo::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
pub use steer_target::*;
use sweet::prelude::RandomSource;

type M = GroupSteerAgent;

/// Add all systems and types for the steer actions:
/// - [`SteerTarget`]
/// - [`MaxForce`]
/// - [`MaxSpeed`]
/// - [`ArriveRadius`]
/// - [`GroupSteerAgent`]
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
