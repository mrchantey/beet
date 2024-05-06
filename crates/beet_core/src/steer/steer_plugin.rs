use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use forky_bevy::extensions::AppExt;

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
			SucceedOnArrive,
			FindSteerTarget,
			ScoreSteerTarget,
			DespawnSteerTarget,
		)>::default());

		let world = app.world_mut();
		world.init_bundle::<SteerBundle>();

		let mut registry =
			world.get_resource::<AppTypeRegistry>().unwrap().write();
		registry.register::<SteerTarget>();
		registry.register::<MaxForce>();
		registry.register::<MaxSpeed>();
		registry.register::<ArriveRadius>();
		registry.register::<WanderParams>();

		drop(registry);

		app.__()
			.add_systems(
				Update,
				(integrate_force, rotate_to_velocity_2d)
					.chain()
					.in_set(PostTickSet),
			)
			.__();

		#[cfg(feature = "gizmos")]
		app.add_systems(Update, debug_group_steer.in_set(PostTickSet));
	}
}

#[cfg(feature = "gizmos")]
pub fn debug_group_steer(
	mut gizmos: Gizmos,
	query: Query<(&Transform, &GroupParams)>,
) {
	for (transform, params) in query.iter() {
		gizmos.circle_2d(
			transform.translation.xy(),
			params.separate_radius,
			Color::hsl(0., 1., 0.5),
		);
		gizmos.circle_2d(
			transform.translation.xy(),
			params.align_radius,
			Color::hsl(30., 1., 0.5),
		);
		gizmos.circle_2d(
			transform.translation.xy(),
			params.cohere_radius,
			Color::hsl(60., 1., 0.5),
		);
	}
}
