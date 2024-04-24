use crate::prelude::*;
use bevy::prelude::*;


/// The distance at which an agent should begin to slow down, defaults to `0.5`
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct GroupParams {
	pub influence_radius: f32,
}

impl Default for GroupParams {
	fn default() -> Self {
		Self {
			influence_radius: 0.5,
		}
	}
}

impl GroupParams {
	pub fn scaled_to(mut self, val: f32) -> Self {
		self.influence_radius *= val;
		self
	}
}

/// Calculate an align impulse
/// as described [here](https://youtu.be/fWqOdLI944M?list=PLRqwX-V7Uu6YHt0dtyf4uiw8tKOxQLvlW&t=349)
pub fn align_impulse(
	position: &Vec3,
	max_speed: &MaxSpeed,
	max_force: &MaxForce,
	params: &GroupParams,
	agents: impl IntoIterator<Item = (&Transform, &Velocity)>,
) -> Impulse {
	let mut average = Vec3::default();
	let mut total = 0;
	for (transform, velocity) in agents.into_iter() {
		if transform.translation == *position
			|| Vec3::distance_squared(*position, transform.translation)
				> params.influence_radius * params.influence_radius
		{
			continue;
		}
		average += velocity.0;
		total += 1;
	}

	if total > 0 {
		average /= total as f32;
		average = average.normalize() * **max_speed;
		average = average.clamp_length_max(**max_force);
	}
	Impulse(average)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;


	fn spawn(world: &mut World, pos: Vec3, vel: Vec3) -> Entity {
		world
			.spawn((Transform::from_translation(pos), Velocity(vel)))
			.id()
	}

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();

		spawn(&mut world, Vec3::ZERO, Vec3::new(1., 0., 0.));
		spawn(&mut world, Vec3::new(1., 0., 0.), Vec3::new(1., 0., 0.));
		spawn(&mut world, Vec3::new(1., 0., 0.), Vec3::new(0., 1., 0.));
		spawn(&mut world, Vec3::new(100., 0., 0.), Vec3::new(30., 0., 0.));

		let mut agents = world.query::<(&Transform, &Velocity)>();
		let agents = agents.iter(&world);

		expect(align_impulse(
			&Vec3::ZERO,
			&MaxSpeed(2.),
			&MaxForce(10.),
			&GroupParams::default(),
			agents,
		))
		.map(|i| i.0)
		.to_be_close_to(Vec3::new(1.41, 1.41, 0.))?;

		Ok(())
	}
}
