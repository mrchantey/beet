//! Behaviors in state machines can be triggered
//! by multiple sources from arbitary positions in a graph.
//! In beet this is achieved using the [`RunNext`] action.
use beet::prelude::*;

#[rustfmt::skip]
fn main() {
	let mut app = App::new();
  app
		.add_plugins((BeetFlowPlugin::default(), BeetDebugPlugin::default()));
	let world = app.world_mut();


	let state2 = world.spawn((
		Name::new("state2"),
		EndOnRun(SUCCESS),
	)).id();

	// transitions are just behaviors that always trigger the next behavior
	let transition = world.spawn((
		Name::new("transition"),
		EndOnRun(SUCCESS),
		RunNext::new(state2),
	)).id();

	world.spawn((
		Name::new("state1"),
		EndOnRun(SUCCESS),
		// here RunOnRunResult can be swapped out with a control flow action
		// that decides which state to go to next
		RunNext::new(transition),
	)).trigger_payload(RUN).flush();
}
