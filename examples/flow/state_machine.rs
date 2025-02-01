//! Behaviors in state machines can be triggered
//! by multiple sources from arbitary positions in a graph.
//! In beet this is achieved using the [`RunOnRunResult`] action.
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	let mut app = App::new();
  app
		.add_plugins(BeetFlowPlugin::default().log_on_run());
	let world = app.world_mut();
		
	
	let state2 = world.spawn((
		Name::new("state2"),
		EndOnRun::success(),
	)).id();

	// transitions are just behaviors that always trigger the next behavior
	let transition = world.spawn((
		Name::new("transition"),
		EndOnRun::success(),
		RunOnRunResult::new_with_target(state2),
	)).id();

	world.spawn((
		Name::new("state1"),
		EndOnRun::success(),
		// here RunOnRunResult can be swapped out with a control flow action
		// that decides which state to go to next
		RunOnRunResult::new_with_target(transition),
	)).flush_trigger(OnRun);
}
