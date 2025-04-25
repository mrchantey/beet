use bevy::prelude::*;
use std::f32::consts::TAU;
use sweet::prelude::When;



#[derive(Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Collectable;


const TURNS_PER_SECOND: f32 = 0.5;

pub fn rotate_collectables(
	time: When<Time>,
	mut query: Populated<&mut Transform, With<Collectable>>,
) {
	for mut transform in query.iter_mut() {
		let angle = time.delta_secs() * TAU * TURNS_PER_SECOND;
		transform.rotate_y(angle);
	}
}
