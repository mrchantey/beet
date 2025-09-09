use crate::prelude::*;
use bevy::prelude::*;

/// Calculate an align impulse
/// as described [here](https://youtu.be/fWqOdLI944M?list=PLRqwX-V7Uu6YHt0dtyf4uiw8tKOxQLvlW&t=349).
pub fn align_impulse<'a, T>(
	target_entity: Entity,
	position: Vec3,
	align: &Align<T>,
	agents: impl IntoIterator<Item = (Entity, &'a Transform, &'a Velocity)>,
) -> Impulse {
	let mut average = Vec3::default();
	let mut total = 0;
	for (entity, transform, velocity) in agents.into_iter() {
		if entity == target_entity
			|| Vec3::distance_squared(position, transform.translation)
				> align.radius * align.radius
		{
			continue;
		}
		average += velocity.0;
		total += 1;
	}

	if total > 0 {
		average /= total as f32;
	}
	Impulse(average * align.scalar)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;


	fn spawn(world: &mut World, pos: Vec3, vel: Vec3) -> Entity {
		world
			.spawn((Transform::from_translation(pos), Velocity(vel)))
			.id()
	}

	#[test]
	fn aligns() {
		let mut world = World::new();

		let entity = spawn(&mut world, Vec3::ZERO, Vec3::new(1., 0., 0.));
		spawn(&mut world, Vec3::ZERO, Vec3::new(1., 0., 0.));
		spawn(&mut world, Vec3::ZERO, Vec3::new(0., 1., 0.));
		spawn(&mut world, Vec3::new(100., 0., 0.), Vec3::new(30., 0., 0.));

		let mut agents = world.query::<(Entity, &Transform, &Velocity)>();
		let agents = agents.iter(&world);

		align_impulse(
			entity,
			Vec3::ZERO,
			&Align::<GroupSteerAgent>::default(),
			agents,
		)
		.xpect()
		.map(|i| i.0)
		.to_be(Vec3::new(0.5, 0.5, 0.));
		// .xpect_close(Vec3::new(1.41, 1.41, 0.))?;
	}
}
