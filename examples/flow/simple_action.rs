use beet::prelude::*;
// flush_trigger utils

#[action(log_on_run)]
#[derive(Component)]
struct LogOnRun(pub String);

fn log_on_run(ev: On<GetOutcome>, query: Query<&LogOnRun>) {
	let name = query
		// ensure that we use ev.event_target(), wich is the 'action entity'
		// ev.target() is the 'action observer'
		.get(ev.action())
		// common pattern for getting an action,
		// it should never be missing
		.expect(&expect_action::to_have_action(&ev));
	println!("running: {}", name.0);
}

fn main() {
	App::new()
		.add_plugins(ControlFlowPlugin::default())
		.world_mut()
		.spawn(LogOnRun("root".to_string()))
		.trigger_target(GetOutcome)
		.flush();
	println!("done!");
}
