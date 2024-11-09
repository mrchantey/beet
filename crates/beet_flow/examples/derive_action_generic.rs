use beet_flow::prelude::*;
use bevy::prelude::*;

#[derive(Component, Action)]
#[observers(log_name_on_run, log_on_run::<T>)]
struct LogOnRun<T: 'static + Send + Sync + ToString>(pub T);

fn log_on_run<T: 'static + Send + Sync + ToString>(
	trigger: Trigger<OnRun>,
	query: Populated<&LogOnRun<T>>,
) {
	let name = query
		.get(trigger.entity())
		.map(|n| n.0.to_string())
		.unwrap();
	println!("log_on_run: {name}");
}

fn log_name_on_run(trigger: Trigger<OnRun>, query: Populated<&Name>) {
	let name = query
		.get(trigger.entity())
		.map(|n| n.as_str())
		.unwrap_or("unnamed");
	println!("log_name_on_run: {name}");
}

fn main() {
	let mut world = World::new();
	world
		.spawn((Name::new("root1"), LogOnRun("root2".to_string())))
		.flush_trigger(OnRun);
}
