use beet_flow::prelude::*;
use bevy::prelude::*;


#[derive(Component, Action)]
#[systems(log_on_run)]
#[category(ActionCategory::Agent)]
struct LogOnRun(pub String);

fn log_on_run(query: Single<&LogOnRun, Added<Running>>) {
	println!("log_on_run: {}", &query.0);
}

fn main() {
	let mut world = World::new();
	world.spawn((Running, LogOnRun("root".to_string())));
}
