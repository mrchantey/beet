use beet::prelude::*;
use bevy::prelude::*;
// flush_trigger utils
use sweet::prelude::EntityWorldMutwExt;

#[action(log_on_run)]
#[derive(Component)]
struct LogOnRun(pub String);

fn log_on_run(ev: Trigger<OnRun>, query: Query<&LogOnRun>) {
	let name = query
		// ensure that we use ev.action, wich is the 'action entity'
		// ev.target() is the 'action observer'
		.get(ev.action)
		// common pattern for getting an action,
		// it should never be missing
		.expect(&expect_action::to_have_action(&ev));
	println!("running: {}", name.0);
}

fn main() {
	App::new()
		.add_plugins(BeetFlowPlugin::default())
		.world_mut()
		.spawn(LogOnRun("root".to_string()))
		.flush_trigger(OnRun::local());
}
