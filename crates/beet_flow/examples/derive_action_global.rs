use beet_flow::prelude::*;
use bevy::ecs::component::StorageType;
use bevy::prelude::*;

#[derive(GlobalAction)]
#[storage(StorageType::SparseSet)]
#[observers(log_name_on_run, log_name_on_run)]
struct LogOnRun(pub String);

fn log_name_on_run(trigger: Trigger<OnAction>, query: Populated<&LogOnRun>) {
	let name = query.get(trigger.action).unwrap();
	println!("log_name_on_run: {}", name.0);
}

fn main() {
	App::new()
		.add_plugins(on_run_global_plugin)
		.world_mut()
		.spawn(LogOnRun("root".to_string()))
		.flush_trigger(OnRunGlobal::default());
}
