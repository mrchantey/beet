use beet::prelude::*;
use bevy::prelude::*;

fn main() {
	std::env::set_var("RUST_LOG", "info");
	pretty_env_logger::init();

	World::new()
		// ensures RunResult from children bubble up to the parent
		.with_observer(bubble_run_result)
		// logs the name of each entity as it runs
		.with_observer(log_name_on_run)
		// create the root entity
		.spawn((Name::new("root"), SequenceFlow))
		.with_children(|parent| {
			parent.spawn((Name::new("child1"), EndOnRun::success()));
			parent.spawn((Name::new("child2"), EndOnRun::success()));
		})
		// trigger OnRun for the root
		.flush_trigger(OnRun);
}
