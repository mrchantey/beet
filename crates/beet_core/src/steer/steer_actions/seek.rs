use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;




#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
/// Go to the agent's [`SteerTarget`] with an optional [`ArriveRadius`]
pub struct Seek;

// TODO if target has Velocity, pursue
fn seek(
	transforms: Query<&Transform>,
	mut agents: Query<(
		&Transform,
		&Velocity,
		&SteerTarget,
		&MaxSpeed,
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
			mut impulse,
			arrive_radius,
		)) = agents.get_mut(**target)
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
					arrive_radius.copied(),
				);
			}
		}
	}
}


impl ActionMeta for Seek {
	fn graph_role(&self) -> GraphRole { GraphRole::Agent }
}

impl ActionSystems for Seek {
	fn systems() -> SystemConfigs { seek.in_set(TickSet) }
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

		app.add_plugins(DefaultBeetPlugins::default())
			.insert_time();


		let tree = Seek.into_beet_builder().build(app.world_mut()).value;

		let agent = app
			.world_mut()
			.spawn((
				TransformBundle::default(),
				ForceBundle::default(),
				SteerBundle::default().with_target(Vec3::new(1.0, 0., 0.)),
			))
			.add_child(tree)
			.id();


		app.update();
		app.update_with_secs(1);

		expect(&app)
			.component::<Transform>(agent)?
			.map(|t| t.translation)
			.to_be(Vec3::new(0.02, 0., 0.))?;


		Ok(())
	}
}
