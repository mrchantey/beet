#![allow(unused)]
use crate::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;

/// Calculate a wander impulse
/// as described [here](https://youtu.be/ujsR2vcJlLk?list=PLRqwX-V7Uu6ZV4yEcW3uDwOgGXKUUsPOM&t=249)
/// except using the original Reynolds approach which works for 2d and 3d
pub fn wander_impulse(
	position: &Vec3,
	velocity: &Velocity,
	wander: &mut Wander,
	max_speed: MaxSpeed,
	rng: &mut impl Rng,
) -> Impulse {
	let inner_delta = Vec3::random_in_sphere(rng) * wander.inner_radius;
	// for the first iteration, last_local_target is Vec3::ZERO, this is
	// allowed and means the first target will be a random point
	let local_target = (wander.last_local_target + inner_delta)
		.normalize_or_zero()
		* wander.outer_radius;
	wander.last_local_target = local_target;

	let global_target = *position
		+ velocity.normalize_or_zero() * wander.outer_distance
		+ local_target;

	let mut impulse =
		seek_impulse(position, velocity, &global_target, max_speed, None);
	*impulse *= wander.scalar;
	impulse
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	#[ignore = "get random"]
	fn works() {
		let mut source = RandomSource::from_seed(0);

		let impulse = wander_impulse(
			&Vec3::default(),
			&Velocity::default(),
			&mut Wander::default(),
			MaxSpeed::default(),
			&mut source.0,
		);
		expect(*impulse).to_be(Vec3::ZERO);
	}
}
