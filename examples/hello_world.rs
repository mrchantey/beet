//! In this example we will create an action
//! and then combine it with some built-in actions to run a behavior.
use beet::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;


// Actions are simply a component-system pair
#[derive(Component)]
struct LogOnRun(pub String);

fn log_on_run(query: Query<&LogOnRun, Added<Running>>) {
	for action in query.iter() {
		log::info!("{}", action.0);
	}
}

fn main() {
	let mut app = App::new();

	app
		// The `LifecyclePlugin` cleans up run state
		.add_plugins((LogPlugin::default(), LifecyclePlugin::default()))
		// action systems are usually added to the `TickSet`
		.add_systems(Update, log_on_run.in_set(TickSet));

	// Behavior graphs are regular entity hierarchies
	app.world_mut()
		.spawn((SequenceSelector::default(), Running))
		.with_children(|parent| {
			parent.spawn((
				LogOnRun("Hello".into()),
				InsertOnRun(RunResult::Success),
			));
			parent.spawn((
				LogOnRun("World".into()),
				InsertOnRun(RunResult::Success),
			));
		});

	// 1 - Selector chooses first child
	app.update();

	// 2 - First child runs >> Hello
	app.update();

	// 3 - Selector chooses second child
	app.update();

	// 4 - Second child runs >> World
	app.update();

	// 5 - Selector succeeds, all done
	app.update();
}