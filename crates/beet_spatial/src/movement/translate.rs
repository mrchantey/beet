use beet_flow::prelude::*;
use bevy::prelude::*;

/// Applies constant translation, multiplied by [`Time::delta_secs`]
///
///
///
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Translate {
	/// Translation to apply, in meters per second
	// #[inspector(min=-2., max=2., step=0.1)]
	pub translation: Vec3,
}

impl Translate {
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
