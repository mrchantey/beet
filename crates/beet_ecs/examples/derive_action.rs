use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Action)]
#[observers(log_name_on_run, log_name_on_run)]
struct LogOnRun(pub String);

fn log_name_on_run(trigger: Trigger<OnRun>, query: Query<&LogOnRun>) {
	let name = query.get(trigger.entity()).map(|n| n.0.as_str()).unwrap();
	println!("log_name_on_run: {name}");
}

fn main() {
	let mut world = World::new();
	world
		.spawn(LogOnRun("root".to_string()))
		.flush_trigger(OnRun);
}
