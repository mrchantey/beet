use beet_core::prelude::When;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::f32::consts::TAU;


/// Translates the agent up and down in a sine wave.
/// ## Tags
/// - [LongRunning](ActionTag::LongRunning)
/// - [MutateOrigin](ActionTag::MutateOrigin)
/// ## Example
/// Hovers up and down every two seconds, at a height of 0.1 meters.
/// ```
/// # use beet_spatial::doctest::*;
/// # let mut world = world();
///	world.spawn((
/// 	Transform::default(),
///		Hover::new(2.,0.1),
///		))
///		.trigger(OnRun::local());
/// ```
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Hover {
	/// Measured in Hz, defaults to 1
	// #[inspector(min = 0.1, max = 3., step = 0.1)]
	pub speed: f32,
	/// Measured in meters, defaults to 1
	// #[inspector(min = 0.1, max = 3., step = 0.1)]
	pub height: f32,
}

impl Default for Hover {
	fn default() -> Self {
		Self {
			speed: 1.,
			height: 1.,
		}
	}
}

impl Hover {
	/// Create a new hover action with the given speed and height.
	pub fn new(speed: f32, height: f32) -> Self { Self { speed, height } }
}

pub(crate) fn hover(
	mut _commands: Commands,
	time: When<Res<Time>>,
	actions: Populated<(&Running, &Hover)>,
	mut transforms: Query<&mut Transform>,
) {
	for (running, hover) in actions.iter() {
		let elapsed = time.elapsed_secs();
		let y = f32::sin(TAU * elapsed * hover.speed) * hover.height;
		transforms
			.get_mut(running.origin)
			.expect(&expect_action::to_have_origin(&running))
			.translation
			.y = y;
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
			.spawn((Transform::default(), Hover::default()))
			.flush_trigger(OnRun::local())
			.id();

		// the 'top' of a sine wave is a quarter of 1 hz
		app.update_with_millis(250);

		app.world()
			.get::<Transform>(agent)
			.unwrap()
			.translation
			.xpect()
			.to_be(Vec3::new(0., 1., 0.));
	}
}
