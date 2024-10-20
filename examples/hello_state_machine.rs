//! Behaviors in state machines can be triggered
//! by multiple sources from arbitary positions in a graph.
//! In beet this is achieved using the [`RunOnRunResult`] action.
use beet::prelude::*;
use beet_examples::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	let mut app = App::new();
  app.insert_resource(BeetDebugConfig::default())
		.add_plugins(minimal_beet_example_plugin);
	let world = app.world_mut();
	let say_world = world.spawn((
		Name::new("World"), 
	)).id();
	
	world.spawn((
		Name::new("Hello"),
		EndOnRun::success(), 
		RunOnRunResult::new_with_target(say_world),
	)).flush_trigger(OnRun);

	world.spawn((
		Name::new("G'day"),
		EndOnRun::success(), 
		RunOnRunResult::new_with_target(say_world),
	)).flush_trigger(OnRun);
}
