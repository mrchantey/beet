use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// A steering behaviour: turns the agent's [`SteerContext`] into the [`Impulse`]
/// to apply this frame.
///
/// Implementors are marker components ([`Seek`], [`Flee`], [`Pursue`],
/// [`Evade`]) driven by the shared [`steer_behavior`] system, so adding a
/// behaviour is one impl plus one registration in
/// [`steer_plugin`](crate::steer::steer_plugin).
pub trait SteerBehavior: Component {
	/// The impulse to apply to the agent this frame.
	fn impulse(&self, cx: &SteerContext) -> Impulse;
}

/// The per-frame steering inputs handed to a [`SteerBehavior`], resolved once by
/// [`steer_behavior`] so every behaviour reads the same view of the agent and of
/// its [`SteerTarget`].
pub struct SteerContext {
	/// The agent's world-space position.
	pub position: Vec3,
	/// The agent's current velocity.
	pub velocity: Velocity,
	/// The resolved world-space position of the agent's [`SteerTarget`].
	pub target_position: Vec3,
	/// The target's velocity, zero unless it is an entity that has one. Only the
	/// predictive behaviours ([`Pursue`], [`Evade`]) read it.
	pub target_velocity: Velocity,
	/// The agent's speed cap.
	pub max_speed: MaxSpeed,
	/// The distance at which the agent begins to slow, if it has one.
	pub arrive_radius: Option<ArriveRadius>,
}

/// Drives every [`Running`] `T`: resolves its agent and [`SteerTarget`] into a
/// [`SteerContext`], then applies the impulse `T` derives from it. An
/// unresolvable target defers to the action's [`OnTargetNotFound`].
pub(crate) fn steer_behavior<T: SteerBehavior>(
	mut commands: Commands,
	transforms: Query<&GlobalTransform>,
	velocities: Query<&Velocity>,
	mut agents: AgentQuery<(
		Entity,
		&GlobalTransform,
		&Velocity,
		&SteerTarget,
		&MaxSpeed,
		&mut Impulse,
		Option<&ArriveRadius>,
	)>,
	query: Query<(Entity, &T, &OnTargetNotFound), With<Running>>,
) -> Result {
	for (action, behavior, on_not_found) in query.iter() {
		let (
			agent_entity,
			transform,
			velocity,
			steer_target,
			max_speed,
			mut impulse,
			arrive_radius,
		) = agents.get_mut(action).map_err(|_| {
			bevyhow!(
				"{} action {action}: its resolved steering agent is missing required steering components (Transform/Velocity/MaxSpeed/Impulse/...)",
				core::any::type_name::<T>()
			)
		})?;
		match steer_target.get_position(&transforms) {
			Ok(target_position) => {
				*impulse = behavior.impulse(&SteerContext {
					position: transform.translation(),
					velocity: velocity.clone(),
					target_position,
					target_velocity: steer_target.get_velocity(&velocities),
					max_speed: *max_speed,
					arrive_radius: arrive_radius.copied(),
				});
			}
			Err(err) => {
				on_not_found.apply(&mut commands, action, agent_entity, err)
			}
		}
	}
	Ok(())
}

/// An [`App`] with the steering systems and a [`Time`], the starting point of
/// every behaviour test.
#[cfg(test)]
pub(crate) fn steer_test_app() -> App {
	let mut app = App::new();
	app.add_plugins(BeetSpatialPlugins).init_resource::<Time>();
	app
}

/// Spawn a steering agent at the origin, carrying `action` and already
/// [`Running`], and return it. Shared by the behaviour tests.
#[cfg(test)]
pub(crate) fn spawn_steer_agent(
	world: &mut World,
	action: impl Bundle,
	target: SteerTarget,
) -> Entity {
	world
		.spawn((
			Transform::default(),
			ForceBundle::default(),
			SteerBundle::default(),
			target,
			action,
			Running::<Outcome>::new(OutHandler::default()),
		))
		.id()
}
