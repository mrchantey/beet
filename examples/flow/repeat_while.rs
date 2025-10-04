//! A simple 'repeat while' pattern.
use beet::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		BeetFlowPlugin::default(),
		DebugFlowPlugin::default(),
	))
	.world_mut()
	.spawn((
		Name::new("root"),
		Sequence,
		// will repeat while the sequence returns [RunResult::Success]
		Repeat::if_success(),
		children![
			(
				Name::new("fails on third run"),
				// this action behaves as a 'while predicate', it will succeed twice
				// then fail the third time.
				SucceedTimes::new(2),
			),
			(
				// this action would be the thing you want to do n times
				// it will only run twice
				Name::new("some action to perform"),
				EndWith(Outcome::Pass),
			)
		],
	))
	.trigger_payload(GetOutcome);
	app.update();
	app.update();
	println!("done, subsequent updates will have no effect");
	app.update();
	app.update();
	app.update();
}
