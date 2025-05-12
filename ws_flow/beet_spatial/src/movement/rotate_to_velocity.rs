use crate::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;
use sweet::prelude::*;

/// Rotate an entity to face its [`Velocity`] in 2D space
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct RotateToVelocity2d;


pub(crate) fn rotate_to_velocity_2d(
	mut query: Query<(&mut Transform, &Velocity), With<RotateToVelocity2d>>,
) {
	for (mut transform, velocity) in query.iter_mut() {
		let Some(dir) = velocity.0.try_normalize() else {
			continue;
		};
		transform.rotation =
			Quat::from_rotation_z(f32::atan2(dir.y, dir.x) - PI * 0.5);
	}
}


/// Rotate an entity to face its [`Velocity`] in 3D space
/// If the velocity is zero, this does nothing
#[derive(Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
pub struct RotateToVelocity3d(pub f32);

impl Default for RotateToVelocity3d {
	fn default() -> Self { Self(5.) }
}

pub(crate) fn rotate_to_velocity_3d(
	time: When<Res<Time>>,
	mut query: Query<(&mut Transform, &Velocity, &RotateToVelocity3d)>,
) {
	for (mut transform, velocity, rotate) in query.iter_mut() {
		let Some(dir) = velocity.0.try_normalize() else {
			continue;
		};
		transform.rotation = transform
			.rotation
			.slerp(Quat::look_at(dir), **rotate * time.delta_secs());
	}
}
