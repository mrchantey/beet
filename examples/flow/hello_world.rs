//! A basic behavior tree sequence example
use beet::prelude::*;

#[rustfmt::skip]
fn main() {
	App::new()
		.add_plugins((
			BeetFlowPlugin::default(),
			DebugFlowPlugin::default()
		))
		.world_mut()
		.spawn((
			Name::new("root"),
			Sequence
		))
		.with_child((
			Name::new("child1"),
			EndWith(Outcome::Pass),
		))
		.with_child((
			Name::new("child2"),
			EndWith(Outcome::Pass),
		))
		.trigger_action(GetOutcome).flush();
}
