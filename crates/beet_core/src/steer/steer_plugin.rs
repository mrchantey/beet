use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

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
		)>::default())
		.register_type::<SteerTarget>()
		.register_type::<MaxForce>()
		.register_type::<MaxSpeed>()
		.register_type::<ArriveRadius>()
		.register_type::<WanderParams>()
		.register_type::<GroupParams>()
		.register_type::<GroupSteerAgent>()
		/*_*/;

		let world = app.world_mut();
		world.init_bundle::<SteerBundle>();

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
