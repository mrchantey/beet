use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone)]
pub struct MovementPlugin;

impl Plugin for MovementPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ActionPlugin::<(
			Hover,
			Translate,
			SetAgentOnRun<Velocity>,
		)>::default()).add_systems(
			Update,
			(
				integrate_force,
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
		world.init_bundle::<ForceBundle>();
	}
}
