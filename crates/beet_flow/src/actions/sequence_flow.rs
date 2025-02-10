use crate::prelude::*;
use bevy::prelude::*;


#[action(on_start, on_next)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct SequenceFlow;


/// When [`OnRun`] is called, trigger the first child if it exists.
/// Otherwise immediately succeed.
fn on_start(
	trig: Trigger<On<Run>>,
	commands: Commands,
	query: Query<&Children>,
) {
	let children = query
		.get(trig.action)
		.expect(&expect_action::to_have_children(&trig));
	if let Some(first_child) = children.iter().next() {
		trig.trigger_next(commands, *first_child);
	} else {
		trig.trigger_response(commands, RunResult::Success);
	}
}


fn on_next(
	trig: Trigger<On<RunResult>>,
	commands: Commands,
	query: Query<&Children>,
) {
	if trig.payload == RunResult::Failure {
		trig.trigger_bubble(commands);
		return;
	}
	let children = query
		.get(trig.action)
		.expect(&expect_action::to_have_children(&trig));
	let index = children
		.iter()
		.position(|&x| x == trig.prev_action)
		.expect(&expect_action::to_have_child(&trig, trig.prev_action));
	if index == children.len() - 1 {
		trig.trigger_bubble(commands);
	} else {
		trig.trigger_next_with(commands, children[index + 1], Run);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let on_result = observe_trigger_names::<On<RunResult>>(world);
		let on_run = observe_triggers::<On<Run>>(world);

		world
			.spawn((Name::new("root"), SequenceFlow))
			.with_children(|parent| {
				parent.spawn((
					Name::new("child1"),
					RespondWith(RunResult::Success),
				));
				parent.spawn((
					Name::new("child2"),
					RespondWith(RunResult::Success),
				));
			})
			.flush_trigger(Run.trigger());

		expect(&on_run).to_have_been_called_times(6);
		expect(&on_result).to_have_been_called_times(3);
		expect(&on_result).to_have_returned_nth_with(0, &"child1".to_string());
		expect(&on_result).to_have_returned_nth_with(1, &"child2".to_string());
		expect(&on_result).to_have_returned_nth_with(2, &"root".to_string());
	}
}
