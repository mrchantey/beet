use beet::prelude::*;
use bevy::prelude::*;

#[derive(Component, Action)]
#[observers(log_on_run)]
struct LogOnRun(pub String);

fn log_on_run(trigger: Trigger<OnRun>, query: Query<&LogOnRun>) {
	let name = query.get(trigger.entity()).map(|n| n.0.as_str()).unwrap();
	println!("running: {name}");
}

fn main() {
	App::new()
		.add_plugins(ActionPlugin::<LogOnRun>::default())
		.world_mut()
		.spawn(LogOnRun("root".to_string()))
		.flush_trigger(OnRun);
}
