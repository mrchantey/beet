use beet::prelude::*;
use bevy::prelude::*;

// actions are a component-system pair
#[derive(Component, Action)]
#[action(system=log_on_run)]
pub struct LogOnRun(pub String);

fn log_on_run(query: Query<&LogOnRun, Added<Running>>) {
	for action in query.iter() {
		println!("{}", action.0);
	}
}

fn main() {
	let mut app = App::new();

	// the BeetPlugin adds the systems associated with each action,
	// as well as utility systems that clean up run state
	app.add_plugins(BeetSystemsPlugin::<
		(SequenceSelector, LogOnRun, InsertOnRun<RunResult>),
		_,
	>::default());

	// behavior graphs are regular entity hierarchies!
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

	// each update is a tick

	println!("1 - Selector chooses first child");
	app.update();

	println!("2 - First child runs");
	app.update();

	println!("3 - Selector chooses second child");
	app.update();

	println!("4 - Second child runs");
	app.update();

	println!("5 - Selector succeeds, all done");
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
