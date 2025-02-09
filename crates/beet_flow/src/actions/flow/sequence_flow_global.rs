use crate::prelude::*;
use bevy::prelude::*;


/// An action that runs all of its children in order until one fails.
/// - If a child succeeds it will run the next child.
/// - If there are no more children to run it will succeed.
/// - If a child fails it will fail.
#[derive(Default, GlobalAction, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(on_start, on_next)]
pub struct SequenceFlowGlobal;

/// When [`OnRun`] is called, trigger the first child if it exists.
/// Otherwise immediately succeed.
fn on_start(
	trigger: Trigger<OnAction>,
	commands: Commands,
	query: Query<&Children>,
) {
	let children = query.get(trigger.action).expect(child_expect::NO_CHILDREN);
	if let Some(first_child) = children.iter().next() {
		trigger.run_next(commands, *first_child);
	} else {
		trigger.on_result(commands, RunResult::Success);
	}
}


fn on_next(
	trigger: Trigger<OnRunResultGlobal>,
	mut commands: Commands,
	query: Query<&Children>,
) {
	if trigger.event().result == RunResult::Failure {
		// trigger.event_mut().call
		commands
			.entity(trigger.entity())
			.trigger(OnRunResult::failure());
		return;
	}
	// let children = query
	// 	.get(trigger.entity())
	// 	.expect(child_expect::NO_CHILDREN);
	// {
	// 	let index = children
	// 		.iter()
	// 		.position(|&x| x == trigger.event().child())
	// 		.expect(child_expect::NOT_MY_CHILD);
	// 	if index == children.len() - 1 {
	// 		commands
	// 			.entity(trigger.entity())
	// 			.trigger(OnRunResult::success());
	// 	} else {
	// 		commands.entity(children[index + 1]).trigger(OnRun);
	// 	}
	// }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(on_run_global_plugin);
		let world = app.world_mut();
		world.add_observer(bubble_run_result);

		let on_result = observe_trigger_names::<OnRunResultGlobal>(world);
		let on_run = observe_triggers::<OnRunGlobal>(world);

		world
			.spawn((Name::new("root"), SequenceFlowGlobal))
			.with_children(|parent| {
				parent.spawn((Name::new("child1"), EndOnRunGlobal::success()));
				parent.spawn((Name::new("child2"), EndOnRunGlobal::success()));
			})
			.flush_trigger(OnRunGlobal::new());

		expect(&on_run).to_have_been_called_times(3);
		expect(&on_result).to_have_been_called_times(3);
		expect(&on_result).to_have_returned_nth_with(0, &"child1".to_string());
		expect(&on_result).to_have_returned_nth_with(1, &"child2".to_string());
		expect(&on_result).to_have_returned_nth_with(2, &"root".to_string());
	}
}
