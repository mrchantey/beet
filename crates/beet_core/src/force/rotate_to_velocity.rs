use crate::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;

#[derive(Component)]
pub struct RotateToVelocity2d;


pub fn rotate_to_velocity_2d(
	mut query: Query<(&mut Transform, &Velocity), With<RotateToVelocity2d>>,
) {
	for (mut transform, velocity) in query.iter_mut() {
		let dir = velocity.0.try_normalize().unwrap_or(Vec3::X);
		transform.rotation =
			Quat::from_rotation_z(f32::atan2(dir.y, dir.x) - PI * 0.5);
	}
}
