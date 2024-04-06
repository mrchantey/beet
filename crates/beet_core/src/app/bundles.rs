use crate::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

pub fn bee_bundle() -> impl Bundle {
	(
		Name::new("bee"),
		RenderText::new("🐝"),
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

pub fn flower_bundle() -> impl Bundle {
	(
		Name::new("flower"),
		TransformBundle::default(),
		RenderText::new("🌻"),
		RandomizePosition {
			offset: Vec3::new(0., -0.5, 0.),
			scale: Vec3::new(1., 0.5, 0.),
		},
	)
}

pub fn flower_auto_spawn_bundle() -> impl Bundle {
	flower_auto_spawn_bundle_with_duration(Duration::from_secs(2))
}
pub fn flower_auto_spawn_bundle_with_duration(
	duration: Duration,
) -> impl Bundle {
	(
		Name::new("Flower Spawner"),
		AutoSpawn::new(
			BeetSceneSerde::<CoreModule>::new_with_bundle(flower_bundle()),
			duration,
		),
	)
}
