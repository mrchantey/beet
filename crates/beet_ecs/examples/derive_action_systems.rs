use beet_ecs::prelude::*;
use bevy::prelude::*;


#[derive(Action)]
#[systems(log_on_run)]
#[category(ActionCategory::Agent)]
struct LogOnRun(pub String);

fn log_on_run(query: Query<&LogOnRun, Added<Running>>) {
	let name = query.get(trigger.entity()).map(|n| n.0.as_str()).unwrap();
	println!("log_on_run: {name}");
}

fn main() {
	let mut world = World::new();
	world
		.spawn(LogOnRun("root".to_string()))
		.flush_trigger(OnRun);
}
