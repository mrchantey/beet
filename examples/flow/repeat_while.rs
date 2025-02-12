//! A simple 'repeat while' pattern.
use beet::prelude::*;
use bevy::prelude::*;

fn main() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		BeetFlowPlugin::default(),
		BeetDebugPlugin::default(),
	))
	.world_mut()
	.spawn((
		Name::new("root"),
		SequenceFlow,
		// will repeat while the sequence returns [RunResult::Success]
		RepeatFlow::if_success(),
	))
	.with_child((
		Name::new("fails on third run"),
		// this action behaves as a 'while predicate', it will succeed twice
		// then fail the third time.
		SucceedTimes::new(2),
	))
	.with_child((
		// this action would be the thing you want to do n times
		// it will only run twice
		Name::new("some action to perform"),
		ReturnWith(RunResult::Success),
	))
	.trigger(OnRun::local());
	app.update();
	app.update();
	println!("done, subsequent updates will have no effect");
	app.update();
	app.update();
	app.update();
}
