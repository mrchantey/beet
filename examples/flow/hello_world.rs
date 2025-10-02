//! A basic behavior tree sequence example
use beet::prelude::*;
// flush_trigger test utils
use sweet::prelude::EntityWorldMutwExt;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((
			BeetFlowPlugin::default(),
			BeetDebugPlugin::default()
		))
		.world_mut()
		.spawn((
			Name::new("root"), 
			Sequence
		))
		.with_child((
			Name::new("child1"),
			ReturnWith(RunResult::Success),
		))
		.with_child((
			Name::new("child2"),
			ReturnWith(RunResult::Success),
		))
		.trigger_entity(RUN).flush();
}
