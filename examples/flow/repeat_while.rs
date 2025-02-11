//! A simple 'repeat while' pattern.
use beet::prelude::*;
use bevy::prelude::*;

#[rustfmt::skip]
fn main() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		BeetFlowPlugin::default().log_on_run(),
	))
	.world_mut()
	.spawn((
		Name::new("root"), 
		SequenceFlow,
		// will repeat while the sequence returns [RunResult::Success]
		RepeatFlow::if_success()
	))
	.with_child((
		Name::new("fails on third run"), 
		// this action behaves as a 'while predicate', it will succeed twice
		// then fail the third time.
		SucceedTimes::new(2)
	))
	.with_child((
		// this action would be the thing you want to do n times 
		// it will only run twice
		Name::new("some action to perform"), 
		EndOnRun::success()
	))
	.trigger(OnRun);

	app.run();
}
