//! In this example we will create an action
//! and then combine it with some built-in actions to run a behavior.
use beet::prelude::*;
use bevy::prelude::*;


// Actions are a component-system pair
#[derive(Component)]
struct LogOnRun(pub String);

fn log_on_run(query: Query<&LogOnRun, Added<Running>>) {
	for action in query.iter() {
		log::info!("{}", action.0);
	}
}

fn main() {
	let mut app = App::new();

	// this will add some helpers that clean up run state
	app.add_plugins(LifecyclePlugin::default());

	// action systems are usually added to the `TickSet`
	app.add_systems(Update, log_on_run.in_set(TickSet));

	// behavior graphs are regular entity hierarchies
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

	// graph traversals occur on each tick
	log::info!("1 - Selector chooses first child");
	app.update();

	log::info!("2 - First child runs");
	app.update();

	log::info!("3 - Selector chooses second child");
	app.update();

	log::info!("4 - Second child runs");
	app.update();

	log::info!("5 - Selector succeeds, all done");
	app.update();
}

/*
1 - Selector chooses first child
2 - First child runs
Hello
3 - Selector chooses second child
4 - Second child runs
World
5 - Selector succeeds, all done
*/
