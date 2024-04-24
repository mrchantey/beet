use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive_action]
#[action(graph_role=GraphRole::Agent)]
/// Somewhat cohesive random walk
pub struct Wander;

fn wander(
	mut agents: Query<(
		&Transform,
		&Velocity,
		&mut WanderParams,
		&MaxSpeed,
		&MaxForce,
		&mut Impulse,
	)>,
	query: Query<(&TargetAgent, &Wander), (With<Running>, With<Wander>)>,
) {
	for (agent, _) in query.iter() {
		if let Ok((
			transform,
			velocity,
			mut wander,
			max_speed,
			max_force,
			mut impulse,
		)) = agents.get_mut(**agent)
		{
			*impulse = wander_impulse(
				&transform.translation,
				&velocity,
				&mut wander,
				*max_speed,
				*max_force,
			);
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
			SteerPlugin::default(),
			BeetSystemsPlugin::<CoreModule, _>::default(),
		))
		.insert_time();


		let tree = Wander::default()
			.into_beet_builder()
			.build(app.world_mut())
			.value;

		let agent = app
			.world_mut()
			.spawn((
				TransformBundle::default(),
				ForceBundle::default(),
				SteerBundle::default(),
			))
			.add_child(tree)
			.id();

		app.update();
		app.update_with_secs(1);

		expect(&app)
			.component::<Transform>(agent)?
			.map(|t| t.translation)
			.not()
			.to_be(Vec3::ZERO)?;

		Ok(())
	}
}
