use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Flee where the agent's [`SteerTarget`] is *going* rather than where it is,
/// the inverse of [`Pursue`].
///
/// A long-running action: stays [`Running`] while active. Like [`Flee`] it
/// ignores [`ArriveRadius`], and like [`Pursue`] it degenerates to that plain
/// [`Flee`] for a target without a [`Velocity`].
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun, OnTargetNotFound)]
pub struct Evade;

impl SteerBehavior for Evade {
	fn impulse(&self, cx: &SteerContext) -> Impulse {
		evade_impulse(
			&cx.position,
			&cx.velocity,
			&cx.target_position,
			&cx.target_velocity,
			cx.max_speed,
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn flees_where_the_target_is_going() {
		let mut app = steer_test_app();
		let world = app.world_mut();
		// the same setup as the `Pursue` test: a target ahead on X, moving away on Z.
		let target = world
			.spawn((
				GlobalTransform::from(Transform::from_xyz(1., 0., 0.)),
				Velocity(Vec3::new(0., 0., 1.)),
			))
			.id();
		let agent = spawn_steer_agent(world, Evade, target.into());

		app.update_with_secs(1);

		// away from the interception point on both axes, the mirror of `Pursue`
		let translation =
			app.world().get::<Transform>(agent).unwrap().translation;
		(translation.x < 0.).xpect_true();
		(translation.z < 0.).xpect_true();
	}
}
