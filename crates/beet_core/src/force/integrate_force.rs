use crate::prelude::*;
use bevy::prelude::*;


/// Implementation of position, velocity, force integration
/// as described by Daniel Shiffman
/// https://natureofcode.com/vectors/#acceleration
pub fn integrate_force(
	time: Res<Time>,
	mut query: Query<(
		&mut Transform,
		Option<&Mass>,
		&mut Velocity,
		Option<&mut Force>,
		Option<&mut Impulse>,
	)>,
) {
	for (mut transform, mass, mut velocity, mut force, mut impulse) in
		query.iter_mut()
	{
		let mut summed_force = Vec3::ZERO;
		if let Some(force) = force.as_mut() {
			summed_force += ***force * time.delta_seconds();
			***force = Vec3::ZERO;
		}
		if let Some(impulse) = impulse.as_mut() {
			summed_force += ***impulse;
			***impulse = Vec3::ZERO;
		}
		if summed_force != Vec3::ZERO {
			let mass = mass.map(|m| **m).unwrap_or(1.0);
			let acceleration = summed_force / mass;
			**velocity += acceleration;
		}
		if **velocity != Vec3::ZERO {
			transform.translation += **velocity * time.delta_seconds();
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	pub fn works() -> Result<()> {
		let mut app = App::new();

		app.add_plugins(SteerPlugin::default());
		app.insert_time();

		let velocity_entity = app
			.world_mut()
			.spawn((TransformBundle::default(), ForceBundle {
				velocity: Velocity(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();
		let force_entity = app
			.world_mut()
			.spawn((TransformBundle::default(), ForceBundle {
				force: Force(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();
		let impulse_entity = app
			.world_mut()
			.spawn((TransformBundle::default(), ForceBundle {
				impulse: Impulse(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();

		let mass_entity = app
			.world_mut()
			.spawn((TransformBundle::default(), ForceBundle {
				mass: Mass(2.),
				impulse: Impulse(Vec3::new(1., 0., 0.)),
				..default()
			}))
			.id();

		app.update_with_secs(1);

		expect(&app)
			.component::<Transform>(velocity_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.))?;
		expect(&app)
			.component::<Transform>(force_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.))?;
		expect(&app)
			.component::<Transform>(impulse_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.))?;
		expect(&app) // impulses are cleared each frame
			.component(impulse_entity)?
			.to_be(&Impulse(Vec3::ZERO))?;
		expect(&app)
			.component::<Transform>(mass_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(0.5, 0., 0.))?;

		app.update_with_secs(1);

		expect(&app)
			.component::<Transform>(velocity_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(2., 0., 0.))?;
		expect(&app)
			.component::<Transform>(force_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(2., 0., 0.))?;
		expect(&app)
			.component::<Transform>(impulse_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(2., 0., 0.))?;
		expect(&app)
			.component::<Transform>(mass_entity)?
			.map(|t| t.translation)
			.to_be(Vec3::new(1., 0., 0.))?;


		Ok(())
	}
}
