use crate::prelude::*;
use beet_core::prelude::*;

/// Calculate a separation impulse
/// as described [here](https://natureofcode.com/autonomous-agents/#example-59-separation).
pub fn separate_impulse<'a, T>(
	target_entity: Entity,
	position: Vec3,
	max_speed: MaxSpeed,
	separate: &Separate<T>,
	agents: impl IntoIterator<Item = (Entity, &'a Transform)>,
) -> Impulse {
	let mut average = Vec3::default();
	let mut total = 0;
	for (entity, transform) in agents.into_iter() {
		let sq_dist = Vec3::distance_squared(position, transform.translation);
		let sq_max = separate.radius * separate.radius;

		if entity == target_entity || sq_dist > sq_max {
			continue;
		}
		let dir = (position - transform.translation).normalize_or_zero();
		let mag = 1. - sq_dist / sq_max;
		average += dir * mag;
		total += 1;
	}

	if total > 0 {
		average /= total as f32;
		average = average.normalize_or_zero() * *max_speed;
	}
	Impulse(average * separate.scalar)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();


		let entity = world.spawn(Transform::from_translation(Vec3::ZERO)).id();
		world.spawn(Transform::from_translation(Vec3::new(0.05, 0., 0.)));
		world.spawn(Transform::from_translation(Vec3::new(0., 0.05, 0.)));
		world.spawn(Transform::from_translation(Vec3::new(0.2, 0., 0.)));

		let mut agents = world.query::<(Entity, &Transform)>();
		let agents = agents.iter(&world);

		separate_impulse(
			entity,
			Vec3::ZERO,
			MaxSpeed(2.),
			&Separate::<GroupSteerAgent>::default(),
			agents,
		)
		.xpect_close(Vec3::new(-1.6174722, -1.1763434, 0.0));
	}
}
