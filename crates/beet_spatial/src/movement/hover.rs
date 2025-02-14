use beet_flow::prelude::*;
use bevy::prelude::*;
use std::f32::consts::TAU;


/// Translates the agent up and down in a sine wave.
/// ## Tags
/// - [LongRunning](ActionTag::LongRunning)
/// - [MutateOrigin](ActionTag::MutateOrigin)
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Hover {
	/// Measured in Hz
	// #[inspector(min = 0.1, max = 3., step = 0.1)]
	pub speed: f32,
	/// Measured in meters
	// #[inspector(min = 0.1, max = 3., step = 0.1)]
	pub height: f32,
}

impl Hover {
	/// Create a new hover action with the given speed and height.
	pub fn new(speed: f32, height: f32) -> Self { Self { speed, height } }
}

pub(crate) fn hover(
	mut _commands: Commands,
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	query: Query<(&Running, &Hover)>,
) {
	for (running, hover) in query.iter() {
		let elapsed = time.elapsed_secs();
		let y = f32::sin(TAU * elapsed * hover.speed) * hover.height;
		transforms
			.get_mut(running.origin)
			.expect(&expect_action::to_have_origin(&running))
			.translation
			.y = y;
	}
}
