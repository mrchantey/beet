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
		)>::default());

		let world = app.world_mut();
		world.init_bundle::<ForceBundle>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();
		registry.register::<Mass>();
		registry.register::<Velocity>();
		registry.register::<Impulse>();
		registry.register::<Force>();
		registry.register::<RotateToVelocity2d>();
		registry.register::<RotateToVelocity3d>();

		drop(registry);

		app.add_systems(
			Update,
			(
				integrate_force,
				(rotate_to_velocity_2d, rotate_to_velocity_3d),
			)
				.chain()
				.in_set(PostTickSet),
		);
	}
}
