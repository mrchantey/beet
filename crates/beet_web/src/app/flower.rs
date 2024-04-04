use beet::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

pub fn flower_auto_spawn(world: &mut World) {
	flower_auto_spawn_with_duration(world, Duration::from_secs(2));
}
pub fn flower_auto_spawn_with_duration(world: &mut World, duration: Duration) {
	let scene = BeetSceneSerde::<CoreModule>::new_with_bundle(flower_bundle());
	world.spawn(AutoSpawn::new(scene, duration));
}


pub fn spawn_flower(world: &mut World) { world.spawn(flower_bundle()); }

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
