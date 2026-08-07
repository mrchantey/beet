use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Move directly away from the agent's [`SteerTarget`], the inverse of [`Seek`].
///
/// A long-running action: stays [`Running`] while active, steering the agent away
/// from its [`SteerTarget`] every frame. [`ArriveRadius`] is ignored, an agent
/// fleeing does not slow as it nears what it is fleeing.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun, OnTargetNotFound)]
pub struct Flee;

impl SteerBehavior for Flee {
	fn impulse(&self, cx: &SteerContext) -> Impulse {
		flee_impulse(
			&cx.position,
			&cx.velocity,
			&cx.target_position,
			cx.max_speed,
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn works() {
		let mut app = steer_test_app();
		let agent = spawn_steer_agent(
			app.world_mut(),
			Flee,
			Vec3::new(1.0, 0., 0.).into(),
		);

		app.update_with_secs(1);

		// directly away from the target, the mirror of the equivalent `Seek`
		app.world()
			.get::<Transform>(agent)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(-0.01, 0., 0.));
	}
}
