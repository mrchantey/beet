//! A basic behavior tree example
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
    .insert_resource(BeetDebugConfig::default())
		.add_plugins(minimal_beet_example_plugin)
		.world_mut()
		.spawn((
			Name::new("root"), 
			SequenceFlow
		))
		.with_children(|parent| {
			parent.spawn((
				Name::new("child1"),
				EndOnRun::success(),
			));
			parent.spawn((
				Name::new("child2"),
				EndOnRun::success(),
			));
		})
		.flush_trigger(OnRun);
}
