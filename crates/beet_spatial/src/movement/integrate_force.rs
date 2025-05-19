use crate::prelude::*;
use bevy::prelude::*;
use sweet::prelude::When;


/// Implementation of position, velocity, force integration
/// as described by Daniel Shiffman
/// https://natureofcode.com/vectors/#acceleration
pub fn integrate_force(
	time: When<Res<Time>>,
	mut query: Populated<(
		&mut Transform,
		&mut Velocity,
		Option<&Mass>,
		Option<&VelocityScalar>,
		Option<&mut Force>,
		Option<&mut Impulse>,
		Option<&mut MaxForce>,
		Option<&mut MaxSpeed>,
	)>,
) {
	for (
		mut transform,
		mut velocity,
		mass,
		scalar,
		mut force,
		mut impulse,
		max_force,
		max_velocity,
	) in query.iter_mut()
	{
		let mut summed_force = Vec3::ZERO;
		if let Some(force) = force.as_mut() {
			summed_force += ***force * time.delta_secs();
			***force = Vec3::ZERO;
		}
		if let Some(impulse) = impulse.as_mut() {
			summed_force += ***impulse;
			***impulse = Vec3::ZERO;
		}
		if summed_force != Vec3::ZERO {
			if let Some(max_force) = max_force {
				summed_force = summed_force.clamp_length_max(**max_force);
			}
			let mass = mass.map(|m| **m).unwrap_or(1.0);
			let acceleration = summed_force / mass;
			**velocity += acceleration;
		}
		if let Some(scalar) = scalar {
			**velocity *= **scalar;
		}
		if **velocity != Vec3::ZERO {
			if let Some(max_velocity) = max_velocity {
				**velocity = velocity.0.clamp_length_max(**max_velocity);
			}
			transform.translation += **velocity * time.delta_secs();
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	pub fn works() {
		let mut app = App::new();

		app.add_systems(Update, integrate_force);
		app.insert_time();

		let velocity_entity = app
			.world_mut()
			.spawn((Transform::default(), ForceBundle {
				velocity: Velocity(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();
		let force_entity = app
			.world_mut()
			.spawn((Transform::default(), ForceBundle {
				force: Force(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();
		let impulse_entity = app
			.world_mut()
			.spawn((Transform::default(), ForceBundle {
				impulse: Impulse(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();

		let mass_entity = app
			.world_mut()
			.spawn((Transform::default(), ForceBundle {
				mass: Mass(2.),
				impulse: Impulse(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();

		app.update_with_secs(1);

		expect(app.world())
			.component::<Transform>(velocity_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.));
		expect(app.world())
			.component::<Transform>(force_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.));
		expect(app.world())
			.component::<Transform>(impulse_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.));
		expect(app.world()) // impulses are cleared each frame
			.component::<Impulse>(impulse_entity)
			.to_be(&Impulse(Vec3::ZERO));
		expect(app.world())
			.component::<Transform>(mass_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(0.5, 0., 0.));

		app.update_with_secs(1);

		expect(app.world())
			.component::<Transform>(velocity_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(2., 0., 0.));
		expect(app.world())
			.component::<Transform>(force_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(2., 0., 0.));
		expect(app.world())
			.component::<Transform>(impulse_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(2., 0., 0.));
		expect(app.world())
			.component::<Transform>(mass_entity)
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.));
	}
}
