use crate::prelude::*;
use bevy::prelude::*;

/// Calculate a separation impulse
/// as described [here](https://natureofcode.com/autonomous-agents/#example-59-separation).
pub fn separate_impulse(
	target_entity: Entity,
	position: Vec3,
	max_speed: MaxSpeed,
	params: &GroupParams,
	agents: impl IntoIterator<Item = (Entity, &Transform)>,
) -> Impulse {
	let mut average = Vec3::default();
	let mut total = 0;
	for (entity, transform) in agents.into_iter() {
		let sq_dist = Vec3::distance_squared(position, transform.translation);
		let sq_max = params.separate_radius * params.separate_radius;

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
	Impulse(average)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();


		let entity = world.spawn(Transform::from_translation(Vec3::ZERO)).id();
		world.spawn(Transform::from_translation(Vec3::new(0.05, 0., 0.)));
		world.spawn(Transform::from_translation(Vec3::new(0., 0.05, 0.)));
		world.spawn(Transform::from_translation(Vec3::new(0.2, 0., 0.)));

		let mut agents = world.query::<(Entity, &Transform)>();
		let agents = agents.iter(&world);

		expect(separate_impulse(
			entity,
			Vec3::ZERO,
			MaxSpeed(2.),
			&GroupParams::default(),
			agents,
		))
		.map(|i| i.0)
		.to_be_close_to(Vec3::new(-1.41, -1.41, 0.))?;

		Ok(())
	}
}
