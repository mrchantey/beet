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
#[require(ContinueRun, OnTargetNotFound)]
pub struct Seek;

impl SteerBehavior for Seek {
	fn impulse(&self, cx: &SteerContext) -> Impulse {
		seek_impulse(
			&cx.position,
			&cx.velocity,
			&cx.target_position,
			cx.max_speed,
			cx.arrive_radius,
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn works() {
		let mut app = steer_test_app();
		let agent = spawn_steer_agent(
			app.world_mut(),
			Seek,
			Vec3::new(1.0, 0., 0.).into(),
		);

		app.update_with_secs(1);

		app.world()
			.get::<Transform>(agent)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(0.01, 0., 0.));
	}

	// regression: a steering behaviour-tree nested under a (non-steering) scene root
	// must resolve its agent to the steering entity via an explicit `ActionOf`, not
	// to the scene root (which lacks the steering components) - the seek_3d failure.
	#[beet_core::test]
	fn nested_tree_resolves_agent_and_seeks() {
		let mut app = steer_test_app();
		let world = app.world_mut();

		// the steering agent, nested under a scene root that has no steering bundle.
		let agent = world
			.spawn((
				Transform::default(),
				ForceBundle::default(),
				SteerBundle::default(),
				SteerTarget::Position(Vec3::new(1.0, 0., 0.)),
			))
			.id();
		let scene_root =
			world.spawn(Transform::default()).add_child(agent).id();
		// the behaviour-tree action, a descendant of the agent, acting on it.
		world.spawn((
			ChildOf(agent),
			ActionOf(agent),
			Seek,
			Running::<Outcome>::new(OutHandler::default()),
		));

		app.update_with_secs(1);

		// the seek resolved to the nested agent and moved it...
		app.world()
			.get::<Transform>(agent)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(0.01, 0., 0.));
		// ...not the scene root, which never had a steer target and never moved.
		app.world()
			.get::<Transform>(scene_root)
			.unwrap()
			.translation
			.xpect_eq(Vec3::ZERO);
	}

	// regression: a 3d target offset on multiple axes must move the agent on all of
	// them, not just X (the seek_3d "moves on X only" symptom). The steering math is
	// fully 3d; this guards against any axis being flattened downstream.
	#[beet_core::test]
	fn seeks_on_all_axes() {
		let mut app = steer_test_app();
		let agent = spawn_steer_agent(
			app.world_mut(),
			Seek,
			Vec3::new(1.0, 0., 1.0).into(),
		);

		app.update_with_secs(1);

		let translation =
			app.world().get::<Transform>(agent).unwrap().translation;
		// moved toward the target on both X and Z, symmetrically, not on Y.
		(translation.x > 0.).xpect_true();
		(translation.z > 0.).xpect_true();
		translation.x.xpect_close(translation.z);
		translation.y.xpect_eq(0.);
	}
}
