/*
These algorithms are from the book "The Nature of Code" by Daniel Shiffman
[Daniel Shiffman - The Nature Of Code - Autonomous Agents](https://natureofcode.com/autonomous-agents/)
[Valentino Braitenberg - Experiments in Synthetic Psychology](https://mitpress.mit.edu/9780262521123/)
[Craig Reynolds - Steering Behavior for Autonomous Characters](https://www.red3d.com/cwr/steer/gdc99/)
[Craig Reynolds - References](https://www.red3d.com/cwr/steer/)
*/
use crate::prelude::*;
use bevy::prelude::*;

/// Calculate a seek impulse
/// as described [here](https://www.youtube.com/watch?v=p1Ws1ZhG36g&list=PLRqwX-V7Uu6ZV4yEcW3uDwOgGXKUUsPOM&t=585s)
pub fn seek_impulse(
	position: &Vec3,
	velocity: &Velocity,
	target_position: &Vec3,
	max_speed: MaxSpeed,
	arrive_radius: Option<ArriveRadius>,
) -> Impulse {
	let desired_speed =
		arrive_speed(position, target_position, max_speed, arrive_radius);

	let delta_position = *target_position - *position;
	let desired_velocity = delta_position.normalize_or_zero() * *desired_speed;

	let impulse = desired_velocity - **velocity;

	Impulse(impulse)
}
/// Inverse of [`seek_impulse`]
/// as described [here](https://youtu.be/Q4MU7pkDYmQ?list=PLRqwX-V7Uu6ZV4yEcW3uDwOgGXKUUsPOM&t=179)
pub fn flee_impulse(
	position: &Vec3,
	velocity: &Velocity,
	target_position: &Vec3,
	max_speed: MaxSpeed,
) -> Impulse {
	let mut impulse =
		seek_impulse(position, velocity, target_position, max_speed, None);
	*impulse *= -1.0;
	impulse
}

/// Calculate a pursue impulse
/// as described [here](https://youtu.be/Q4MU7pkDYmQ?list=PLRqwX-V7Uu6ZV4yEcW3uDwOgGXKUUsPOM&t=544)
/// Currently the tuning parameter is very coarse, based on distance to target.
/// It assumes the pursuer is moving directly target at 1 m/s
pub fn pursue_impulse(
	position: &Vec3,
	velocity: &Velocity,
	target_position: &Vec3,
	target_velocity: &Velocity,
	max_speed: MaxSpeed,
	arrive_radius: Option<ArriveRadius>,
) -> Impulse {
	let delta_position = *target_position - *position;
	let distance_to_target = delta_position.length();

	let next_target_position =
		*target_position + **target_velocity * distance_to_target;
	seek_impulse(
		position,
		velocity,
		&next_target_position,
		max_speed,
		arrive_radius,
	)
}
/// Calculate an evade impulse
/// as described [here](https://youtu.be/Q4MU7pkDYmQ?list=PLRqwX-V7Uu6ZV4yEcW3uDwOgGXKUUsPOM&t=584)
pub fn evade_impulse(
	position: &Vec3,
	velocity: &Velocity,
	target_position: &Vec3,
	target_velocity: &Velocity,
	max_speed: MaxSpeed,
) -> Impulse {
	let mut impulse = pursue_impulse(
		position,
		velocity,
		target_position,
		target_velocity,
		max_speed,
		None,
	);
	*impulse *= -1.0;
	impulse
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use ::sweet::prelude::*;

	#[test]
	fn algo(){
		let impulse = seek_impulse(
			&Vec3::default(),
			&Velocity::default(),
			&Vec3::new(1., 0., 0.),
			MaxSpeed::default(),
			None,
		);
		expect(*impulse).to_be(Vec3::new(1., 0., 0.));
	}
}
