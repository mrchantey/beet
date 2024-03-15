use crate::prelude::*;
use bevy::prelude::*;
use forky_bevy::extensions::Vec3Ext;

/// The distance at which an agent should begin to slow down, defaults to `0.5`
#[derive(Debug, Clone, PartialEq, Component)]
pub struct WanderParams {
	pub outer_distance: f32,
	pub outer_radius: f32,
	/// This effects the responsiveness of the wander
	pub inner_radius: f32,
	/// Representation of the last target, local to the outer circle
	pub last_local_target: Vec3,
}

impl Default for WanderParams {
	fn default() -> Self {
		Self {
			outer_distance: 1.,
			outer_radius: 0.5,
			inner_radius: 0.01,
			last_local_target: Vec3::ZERO,
		}
	}
}

impl WanderParams {
	pub fn default_forward() -> Self {
		Self {
			last_local_target: Vec3::new(0., 0., -1.),
			..default()
		}
	}
	pub fn default_right() -> Self {
		Self {
			last_local_target: Vec3::RIGHT,
			..default()
		}
	}
}

/// Calculate a wander impulse
/// as described [here](https://youtu.be/ujsR2vcJlLk?list=PLRqwX-V7Uu6ZV4yEcW3uDwOgGXKUUsPOM&t=249)
/// except using the original Reynolds approach which works for 2d and 3d
pub fn wander_impulse(
	position: &Vec3,
	velocity: &Velocity,
	wander: &mut WanderParams,
	max_speed: MaxSpeed,
	max_force: MaxForce,
) -> Impulse {
	let inner_delta = Vec3::random_in_sphere() * wander.inner_radius;
	// for the first iteration, last_local_target is Vec3::ZERO, this is
	// allowed and means the first target will be a random point
	let local_target = (wander.last_local_target + inner_delta)
		.normalize_or_zero()
		* wander.outer_radius;
	wander.last_local_target = local_target;

	let global_target = *position
		+ velocity.normalize_or_zero() * wander.outer_distance
		+ local_target;

	seek_impulse(
		position,
		velocity,
		&global_target,
		max_speed,
		max_force,
		None,
	)
}
