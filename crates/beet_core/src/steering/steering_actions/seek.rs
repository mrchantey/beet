use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use forky_core::ResultTEExt;

#[derive_action]
#[action(graph_role=GraphRole::Agent)]
pub struct Seek;

// TODO if target has Velocity, pursue
fn seek(
	transforms: Query<&Transform>,
	mut targets: Query<(
		&Transform,
		&Velocity,
		&SteerTarget,
		&MaxSpeed,
		&MaxForce,
		&mut Impulse,
		Option<&ArriveRadius>,
	)>,
	query: Query<(&TargetAgent, &Seek), With<Running>>,
) {
	for (target, _) in query.iter() {
		if let Ok((
			transform,
			velocity,
			steer_target,
			max_speed,
			max_force,
			mut impulse,
			arrive_radius,
		)) = targets.get_mut(**target)
		// if agent has no steer_target thats ok
		{
			if let Some(target_position) = steer_target
				.position(&transforms)
				.ok_or(|e| log::warn!("{e}"))
			{
				*impulse = seek_impulse(
					&transform.translation,
					&velocity,
					&target_position,
					*max_speed,
					*max_force,
					arrive_radius.copied(),
				);
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();

		app.add_plugins((
			SteeringPlugin::default(),
			BeetSystemsPlugin::<CoreNode, _>::default(),
		))
		.insert_time();

		let agent = app
			.world
			.spawn((
				TransformBundle::default(),
				ForceBundle::default(),
				SteerBundle::default().with_target(Vec3::new(1.0, 0., 0.)),
			))
			.id();

		Seek.into_beet_builder().spawn(&mut app.world, agent);

		app.update();
		app.update_with_secs(1);

		expect(&app)
			.component::<Transform>(agent)?
			.map(|t| t.translation)
			.to_be(Vec3::new(0.2, 0., 0.))?;


		Ok(())
	}
}
