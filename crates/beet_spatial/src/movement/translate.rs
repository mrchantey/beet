use beet_action::prelude::*;
use beet_core::prelude::When;
use beet_core::prelude::*;

/// Applies constant translation to the agent, multiplied by
/// [`Time::delta_secs`].
///
/// A long-running action: stays [`Running`] while active, translating
/// the agent every frame.
/// ## Example
/// Translates to the right at 1 unit per second while running.
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// # use beet_spatial::prelude::*;
/// # let mut world = World::new();
/// world.spawn((Transform::default(), Translate::new(Vec3::new(1.0, 0., 0.))));
/// ```
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Translate {
	/// Translation to apply, in meters per second
	// #[inspector(min=-2., max=2., step=0.1)]
	pub translation: Vec3,
}

impl Translate {
	/// Create a new translation action with the given translation as units/second.
	pub fn new(translation: Vec3) -> Self { Self { translation } }
}
pub(crate) fn translate(
	time: When<Res<Time>>,
	action: Populated<(Entity, &Translate), With<Running>>,
	mut agents: AgentQuery<&mut Transform>,
) -> Result {
	for (action, translate) in action.iter() {
		agents.get_mut(action)?.translation +=
			translate.translation * time.delta_secs();
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
				Translate::new(Vec3::new(1.0, 0., 0.)),
				Running::<Outcome>::new(OutHandler::default()),
			))
			.id();

		app.update_with_secs(1);

		app.world()
			.get::<Transform>(agent)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(1., 0., 0.));
	}
}
