use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
/// Somewhat cohesive random walk
pub struct Wander {
	/// The scalar to apply to the impulse
	pub scalar: f32,
}

impl Default for Wander {
	fn default() -> Self { Self { scalar: 1. } }
}

impl Wander {
	pub fn new(scalar: f32) -> Self { Self { scalar } }
}

fn wander(
	mut agents: Query<(
		&Transform,
		&Velocity,
		&mut WanderParams,
		&MaxSpeed,
		&mut Impulse,
	)>,
	query: Query<(&TargetAgent, &Wander), (With<Running>, With<Wander>)>,
) {
	for (agent, wander) in query.iter() {
		if let Ok((
			transform,
			velocity,
			mut wander_params,
			max_speed,
			mut impulse,
		)) = agents.get_mut(**agent)
		{
			let new_impulse = wander_impulse(
				&transform.translation,
				&velocity,
				&mut wander_params,
				*max_speed,
			);

			**impulse += *new_impulse * wander.scalar;
		}
	}
}


impl ActionMeta for Wander {
	fn category(&self) -> ActionCategory { ActionCategory::Agent }
}

impl ActionSystems for Wander {
	fn systems() -> SystemConfigs { wander.in_set(TickSet) }
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

		app.add_plugins((LifecyclePlugin, MovementPlugin, SteerPlugin))
			.insert_time();

		let agent = app
			.world_mut()
			.spawn((
				TransformBundle::default(),
				ForceBundle::default(),
				SteerBundle::default(),
			))
			.with_children(|parent| {
				parent.spawn((RootIsTargetAgent, Running, Wander::default()));
			})
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
