// use beet_flow::action_observers;
use beet_flow2::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;


fn log_on_run(trigger: On<Run>, query: Populated<&LogOnRun>) {
	let name = query.get(trigger.action).unwrap();
	println!("log_name_on_run: {}", name.0);
}

fn main() {
	App::new()
		// .add_observer(observer)
		.add_plugins(BeetFlowPlugin::default())
		.world_mut()
		.spawn(LogOnRun("root".to_string()))
		.trigger(Run);
}
