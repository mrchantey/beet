use beet::prelude::*;
use bevy::prelude::*;




pub fn spawn_bee(world: &mut World) { world.spawn(bee_bundle()); }

pub fn bee_bundle() -> impl Bundle {
	(
		Name::new("bee"),
		RenderText::new("🐝"),
		BindAgentToFirstGraph,
		RandomizePosition::default(),
		TransformBundle::default(),
		ForceBundle::default(),
		SteerBundle {
			arrive_radius: ArriveRadius(0.2),
			wander_params: WanderParams {
				outer_distance: 0.2,
				outer_radius: 0.1,
				inner_radius: 0.01, //lower = smoother
				last_local_target: default(),
			},
			max_force: MaxForce(0.1),
			max_speed: MaxSpeed(0.3),
			..default()
		},
	)
}
