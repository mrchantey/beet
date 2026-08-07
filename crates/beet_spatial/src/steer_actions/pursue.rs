use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Intercept the agent's [`SteerTarget`] by steering at where it is *going*
/// rather than where it is, the predictive form of [`Seek`].
///
/// A long-running action: stays [`Running`] while active. The lead is scaled by
/// the distance to the target, so it degenerates to a plain [`Seek`] for a
/// [`SteerTarget::Position`] or any target without a [`Velocity`].
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun, OnTargetNotFound)]
pub struct Pursue;

impl SteerBehavior for Pursue {
	fn impulse(&self, cx: &SteerContext) -> Impulse {
		pursue_impulse(
			&cx.position,
			&cx.velocity,
			&cx.target_position,
			&cx.target_velocity,
			cx.max_speed,
			cx.arrive_radius,
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn leads_a_moving_target() {
		let mut app = steer_test_app();
		let world = app.world_mut();
		// a target ahead on X, moving away on Z. `GlobalTransform` is set
		// explicitly, the test app runs no transform propagation.
		let target = world
			.spawn((
				GlobalTransform::from(Transform::from_xyz(1., 0., 0.)),
				Velocity(Vec3::new(0., 0., 1.)),
			))
			.id();
		let agent = spawn_steer_agent(world, Pursue, target.into());

		app.update_with_secs(1);

		// steered at the predicted interception point, so the agent moves on Z as
		// well as X - a plain `Seek` at the same target would move on X alone.
		let translation =
			app.world().get::<Transform>(agent).unwrap().translation;
		(translation.x > 0.).xpect_true();
		(translation.z > 0.).xpect_true();
	}
}
