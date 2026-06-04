use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Go to the agent's [`SteerTarget`] with an optional [`ArriveRadius`].
///
/// A long-running action: stays [`Running`] while active, steering the
/// agent toward its [`SteerTarget`] every frame. Pair with [`EndOnArrive`]
/// (in a [`Parallel`] or [`Fallback`]) for a terminating sibling.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Seek {
	/// What to do when the target is not found
	// TODO this should be a seperate component used by other actions as well
	pub on_not_found: OnTargetNotFound,
}

impl Seek {
	/// Create a new [`Seek`] with the given [`OnTargetNotFound`] behavior
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
	/// Do nothing
	Ignore,
	/// End the run with [`Outcome::FAIL`]
	Fail,
	/// End the run with [`Outcome::PASS`]
	Succeed,
}


// TODO if target has Velocity, pursue
pub(crate) fn seek(
	mut commands: Commands,
	transforms: Query<&GlobalTransform>,
	mut agents: AgentQuery<(
		Entity,
		&GlobalTransform,
		&Velocity,
		&SteerTarget,
		&MaxSpeed,
		&mut Impulse,
		Option<&ArriveRadius>,
	)>,
	query: Query<(Entity, &Seek), With<Running>>,
) -> Result {
	for (action, seek) in query.iter() {
		let (
			agent_entity,
			transform,
			velocity,
			steer_target,
			max_speed,
			mut impulse,
			arrive_radius,
		) = agents.get_mut(action)?;
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
			(OnTargetNotFound::Ignore, Err(_)) => {}
			(OnTargetNotFound::Warn, Err(msg)) => {
				log::warn!("{}", msg);
			}
			(OnTargetNotFound::Fail, Err(_)) => {
				commands.entity(action).queue(EndRun(Outcome::FAIL));
			}
			(OnTargetNotFound::Succeed, Err(_)) => {
				commands.entity(action).queue(EndRun(Outcome::PASS));
			}
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetSpatialPlugins).init_resource::<Time>();

		let agent = app
			.world_mut()
			.spawn((
				Transform::default(),
				ForceBundle::default(),
				SteerBundle::default(),
				SteerTarget::Position(Vec3::new(1.0, 0., 0.)),
				Seek::default(),
				Running::<Outcome>::new(OutHandler::default()),
			))
			.id();

		app.update_with_secs(1);

		app.world()
			.get::<Transform>(agent)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(0.01, 0., 0.));
	}
}
