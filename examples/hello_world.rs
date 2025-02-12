//! A basic behavior tree sequence example
use beet::prelude::*;
use bevy::prelude::*;
// flush_trigger test utils
use sweet::prelude::EntityWorldMutwExt;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((
			// register the run and result routing observers
			BeetFlowPlugin::default(),
			// this will log any running entity
			BeetDebugPlugin::default()
		))
		.world_mut()
		.spawn((
			Name::new("root"), 
			SequenceFlow
		))
		.with_child((
			Name::new("child1"),
			ReturnWith(RunResult::Success),
		))
		.with_child((
			Name::new("child2"),
			ReturnWith(RunResult::Success),
		))
		.flush_trigger(OnRun::local());
}
