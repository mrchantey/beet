//! Systems and components related to 'unintelligent' movement,
//! for more natural and advanced movement see the [`crate::steer`] module.
mod force_bundle;
pub use self::force_bundle::*;
mod hover;
pub use self::hover::*;
mod integrate_force;
pub use self::integrate_force::*;
mod rotate_to_velocity;
pub use self::rotate_to_velocity::*;
mod translate;
pub use self::translate::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Add all systems and types for the base movement actions:
/// - [`Translate`]
/// - [`Hover`]
/// - [`RotateToVelocity2d`]
/// - [`RotateToVelocity3d`]
pub fn movement_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(
			(
				integrate_force,
				(rotate_to_velocity_2d, rotate_to_velocity_3d),
			)
				.chain()
				.in_set(PostTickSet),
			(hover, translate).in_set(TickSet),
		),
	)
	.register_type::<Mass>()
	.register_type::<Velocity>()
	.register_type::<Impulse>()
	.register_type::<Force>()
	.register_type::<RotateToVelocity2d>()
	.register_type::<RotateToVelocity3d>()
	.register_type::<VelocityScalar>();
	let world = app.world_mut();
	world.register_bundle::<ForceBundle>();
}
