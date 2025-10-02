//! A basic behavior tree sequence example
use beet::prelude::*;

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
			EndOnRun(SUCCESS),
		))
		.with_child((
			Name::new("child2"),
			EndOnRun(SUCCESS),
		))
		.trigger_entity(RUN).flush();
}
