use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Go to the agent's [`SteerTarget`] with an optional [`ArriveRadius`]
#[derive(Debug, Default, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(seek.in_set(TickSet))]
#[require(ContinueRun)]
pub struct Seek {
	// TODO this should be a seperate component used by other actions as well
	pub on_not_found: OnTargetNotFound,
}

impl Seek {
	pub fn new(on_not_found: OnTargetNotFound) -> Self { Self { on_not_found } }
}


#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub enum OnTargetNotFound {
	/// Warn
	#[default]
	Warn,
	/// Remove the [`SteerTarget`]
	Clear,
	/// Remove the [`SteerTarget`] and emit [`OnRunResult::failure()`]
	Fail,
	/// Remove the [`SteerTarget`] and emit [`OnRunResult::success()`]
	Succeed,
	/// Do nothing
	Ignore,
}


// TODO if target has Velocity, pursue
fn seek(
	mut commands: Commands,
	transforms: Query<&GlobalTransform>,
	mut agents: Query<(
		Entity,
		&GlobalTransform,
		&Velocity,
		&SteerTarget,
		&MaxSpeed,
		&mut Impulse,
		Option<&ArriveRadius>,
	)>,
	query: Query<(Entity, &TargetEntity, &Seek), With<Running>>,
) {
	for (entity, target, seek) in query.iter() {
		if let Ok((
			agent_entity,
			transform,
			velocity,
			steer_target,
			max_speed,
			mut impulse,
			arrive_radius,
		)) = agents.get_mut(**target)
		// if agent has no steer_target thats ok
		{
			match (&seek.on_not_found, steer_target.get_position(&transforms)) {
				(_, Ok(target_position)) => {
					*impulse = seek_impulse(
						&transform.translation(),
						&velocity,
						&target_position,
						*max_speed,
						arrive_radius.copied(),
					);
				}
				(OnTargetNotFound::Clear, Err(_)) => {
					commands.entity(agent_entity).remove::<SteerTarget>();
				}
				(OnTargetNotFound::Fail, Err(_)) => {
					commands.entity(agent_entity).remove::<SteerTarget>();
					commands.entity(entity).trigger(OnRunResult::failure());
				}
				(OnTargetNotFound::Succeed, Err(_)) => {
					commands.entity(agent_entity).remove::<SteerTarget>();
					commands.entity(entity).trigger(OnRunResult::success());
				}
				(OnTargetNotFound::Ignore, Err(_)) => {}
				(OnTargetNotFound::Warn, Err(msg)) => {
					log::warn!("{}", msg);
				}
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use beet_flow::prelude::*;
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
				Transform::default(),
				ForceBundle::default(),
				SteerBundle::default(),
				SteerTarget::Position(Vec3::new(1.0, 0., 0.)),
			))
			.with_children(|parent| {
				parent.spawn((
					TargetEntity(parent.parent_entity()),
					Running,
					Seek::default(),
				));
			})
			.id();


		app.update();
		app.update_with_secs(1);

		expect(app.world())
			.component::<Transform>(agent)?
			.map(|t| t.translation)
			.to_be(Vec3::new(0.02, 0., 0.))?;


		Ok(())
	}
}
