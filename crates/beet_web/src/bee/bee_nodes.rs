use beet::action_list;
use beet::prelude::*;
use bevy_time::Time;
use bevy_transform::components::Transform;
use std::f32::consts::TAU;

// for now we need to manually keep in sync with crates/beet_ecs/src/builtin_nodes/builtin_nodes.rs

action_list!(BeeNode, [
	//bee
	Hover,
	//core
	Translate,
	//steer
	Seek,
	Wander,
	FindSteerTarget,
	ScoreSteerTarget,
	SucceedOnArrive,
	//ecs
	EmptyAction,
	SetRunResult,
	ConstantScore,
	SucceedInDuration,
	SequenceSelector,
	FallbackSelector,
	UtilitySelector
]);

#[action(system=hover)]
#[derive(Default)]
pub struct Hover {
	/// Measured in Hz
	pub speed: f32,
	/// Measured in meters
	pub height: f32,
}

impl Hover {
	pub fn new(speed: f32, height: f32) -> Self { Self { speed, height } }
}

fn hover(
	mut _commands: Commands,
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	query: Query<(&TargetAgent, &Hover), With<Running>>,
) {
	for (target, hover) in query.iter() {
		let elapsed = time.elapsed_seconds();
		let y = f32::sin(TAU * elapsed * hover.speed) * hover.height;
		transforms.get_mut(**target).unwrap().translation.y = y;
	}
}
