use crate::prelude::*;
use bevy::prelude::*;

/// Calculate an align impulse
/// as described [here](https://youtu.be/fWqOdLI944M?list=PLRqwX-V7Uu6YHt0dtyf4uiw8tKOxQLvlW&t=349).
pub fn align_impulse(
	target_entity: Entity,
	position: Vec3,
	params: &GroupParams,
	agents: impl IntoIterator<Item = (Entity, &Transform, &Velocity)>,
) -> Impulse {
	let mut average = Vec3::default();
	let mut total = 0;
	for (entity, transform, velocity) in agents.into_iter() {
		if entity == target_entity
			|| Vec3::distance_squared(position, transform.translation)
				> params.align_radius * params.align_radius
		{
			continue;
		}
		average += velocity.0;
		total += 1;
	}

	if total > 0 {
		average /= total as f32;
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

		let entity = spawn(&mut world, Vec3::ZERO, Vec3::new(1., 0., 0.));
		spawn(&mut world, Vec3::ZERO, Vec3::new(1., 0., 0.));
		spawn(&mut world, Vec3::ZERO, Vec3::new(0., 1., 0.));
		spawn(&mut world, Vec3::new(100., 0., 0.), Vec3::new(30., 0., 0.));

		let mut agents = world.query::<(Entity, &Transform, &Velocity)>();
		let agents = agents.iter(&world);

		expect(align_impulse(
			entity,
			Vec3::ZERO,
			&GroupParams::default(),
			agents,
		))
		.map(|i| i.0)
		.to_be(Vec3::new(0.5, 0.5, 0.))?;
		// .to_be_close_to(Vec3::new(1.41, 1.41, 0.))?;

		Ok(())
	}
}
