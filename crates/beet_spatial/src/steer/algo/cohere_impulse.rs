use crate::prelude::*;
use beet_flow::prelude::GenericActionComponent;
use bevy::prelude::*;

/// Calculate a cohesion impulse
/// as described [here](https://natureofcode.com/autonomous-agents/#exercise-515).
pub fn cohere_impulse<'a, T: GenericActionComponent>(
	target_entity: Entity,
	position: Vec3,
	max_speed: MaxSpeed,
	cohere: &Cohere<T>,
	agents: impl IntoIterator<Item = (Entity, &'a Transform)>,
) -> Impulse {
	let mut average = Vec3::default();
	let mut total = 0;
	for (entity, transform) in agents.into_iter() {
		if entity == target_entity
			|| Vec3::distance_squared(position, transform.translation)
				> cohere.radius * cohere.radius
		{
			continue;
		}

		let delta_pos = transform.translation - position;

		average += delta_pos;
		total += 1;
	}

	if total > 0 {
		average /= total as f32;
		average = average.normalize_or_zero() * *max_speed;
	}
	Impulse(average * cohere.scalar)
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use ::sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();


		let entity = world.spawn(Transform::from_translation(Vec3::ZERO)).id();
		world.spawn(Transform::from_translation(Vec3::new(0.1, 0., 0.)));
		world.spawn(Transform::from_translation(Vec3::new(0., 0.1, 0.)));
		world.spawn(Transform::from_translation(Vec3::new(1., 0., 0.)));

		let mut agents = world.query::<(Entity, &Transform)>();
		let agents = agents.iter(&world);

		expect(cohere_impulse(
			entity,
			Vec3::ZERO,
			MaxSpeed(2.),
			&Cohere::<GroupSteerAgent>::default(),
			agents,
		))
		.map(|i| i.0)
		.to_be_close_to(Vec3::new(1.41, 1.41, 0.));
	}
}
