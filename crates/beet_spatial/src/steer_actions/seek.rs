use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

/// Go to the agent's [`SteerTarget`] with an optional [`ArriveRadius`]
/// ## Tags
/// - [LongRunning](ActionTag::LongRunning)
/// - [MutateOrigin](ActionTag::MutateOrigin)
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Seek {
	/// What to do when the target is not found
	// TODO this should be a seperate component used by other actions as well
	pub on_not_found: OnTargetNotFound,
}

impl Seek {
	/// Create a new [`Seek`] action with the given [`OnTargetNotFound`] behavior
	pub fn new(on_not_found: OnTargetNotFound) -> Self { Self { on_not_found } }
}

/// Instructions for how to behave when a specified [`SteerTarget::Entity`] is not found
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
pub(crate) fn seek(
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
	query: Query<(Entity, &Running, &Seek)>,
) {
	for (action, running, seek) in query.iter() {
		let (
			agent_entity,
			transform,
			velocity,
			steer_target,
			max_speed,
			mut impulse,
			arrive_radius,
		) = agents
			.get_mut(running.origin)
			.expect(&expect_action::to_have_origin(&running));
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
				running.trigger_result(
					&mut commands,
					action,
					RunResult::Failure,
				);
			}
			(OnTargetNotFound::Succeed, Err(_)) => {
				commands.entity(agent_entity).remove::<SteerTarget>();
				running.trigger_result(
					&mut commands,
					action,
					RunResult::Success,
				);
			}
			(OnTargetNotFound::Ignore, Err(_)) => {}
			(OnTargetNotFound::Warn, Err(msg)) => {
				log::warn!("{}", msg);
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();

		app.add_plugins((BeetFlowPlugin::default(), BeetSpatialPlugins))
			.insert_time();

		let agent = app
			.world_mut()
			.spawn((
				Transform::default(),
				ForceBundle::default(),
				SteerBundle::default(),
				SteerTarget::Position(Vec3::new(1.0, 0., 0.)),
				Seek::default(),
			))
			.flush_trigger(OnRun::local())
			.id();

		app.update_with_secs(1);

		app.world()
			.get::<Transform>(agent)
			.unwrap()
			.translation
			.xpect()
			.to_be(Vec3::new(0.01, 0., 0.));
	}
}
