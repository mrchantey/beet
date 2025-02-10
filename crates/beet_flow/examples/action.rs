// use beet_flow::action_observers;
use beet_flow::prelude::*;
use bevy::prelude::*;
use sweet::prelude::*;

#[action(log_name_on_run)]
#[derive(Component)]
struct LogOnRun(pub String);
// action_observers!(log_name_on_run);


fn log_name_on_run(
	trigger: Trigger<ActionContext<Run>>,
	query: Populated<&LogOnRun>,
) {
	let name = query.get(trigger.action).unwrap();
	println!("log_name_on_run: {}", name.0);
}

fn main() {
	App::new()
		// .add_observer(observer)
		.add_plugins(BeetFlowPlugin::default())
		.world_mut()
		.spawn(LogOnRun("root".to_string()))
		.flush_trigger(Run.trigger());
}
