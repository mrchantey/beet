use crate::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;

/// Rotate an entity to face its [`Velocity`] in 2D space
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


/// Rotate an entity to face its [`Velocity`] in 3D space
#[derive(Component)]
pub struct RotateToVelocity3d;

pub fn rotate_to_velocity_3d(
	mut query: Query<(&mut Transform, &Velocity), With<RotateToVelocity3d>>,
) {
	for (mut transform, velocity) in query.iter_mut() {
		let dir = velocity.0.try_normalize().unwrap_or(-Vec3::Z);
		transform.rotation =
			Quat::from_rotation_y(f32::atan2(dir.y, dir.x) - PI * 0.5);
	}
}
