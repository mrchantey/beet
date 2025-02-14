use beet_flow::prelude::*;
use bevy::prelude::*;

/// Applies constant translation, multiplied by [`Time::delta_secs`]
/// ## Tags
/// - [LongRunning](ActionTag::LongRunning)
/// - [MutateOrigin](ActionTag::MutateOrigin)
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
	time: Res<Time>,
	action: Query<(&Running, &Translate)>,
	mut transforms: Query<&mut Transform>,
) {
	for (running, translate) in action.iter() {
		let mut transform = transforms
			.get_mut(running.origin)
			.expect(&expect_action::to_have_origin(&running));
		transform.translation += translate.translation * time.delta_secs();
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
				Translate::new(Vec3::new(1.0, 0., 0.)),
			))
			.flush_trigger(OnRun::local())
			.id();

		app.update_with_secs(1);

		expect(app.world().get::<Transform>(agent).unwrap().translation)
			.to_be(Vec3::new(1., 0., 0.));
	}
}
