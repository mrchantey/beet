use crate::prelude::*;
use bevy::prelude::*;

/// An action that runs all of its children in order until one succeeds.
/// - If a child succeeds it succeed.
/// - If a child fails it will run the next child.
/// - If there are no more children to run it will succeed.
#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct FallbackFlow;

fn on_start(ev: Trigger<OnRun>, commands: Commands, query: Query<&Children>) {
	println!("FallbackFlow on_start");
	let children = query
		.get(ev.entity())
		.expect(&expect_action::to_have_children(&ev));
	if let Some(first_child) = children.iter().next() {
		ev.trigger_next(commands, *first_child);
	} else {
		ev.trigger_result(commands, RunResult::Success);
	}
}

fn on_next(ev: Trigger<OnResult>, commands: Commands, query: Query<&Children>) {
	if ev.payload == RunResult::Success {
		ev.trigger_bubble(commands);
		return;
	}
	let children = query
		.get(ev.entity())
		.expect(&expect_action::to_have_children(&ev));

	let index = children
		.iter()
		.position(|&x| x == ev.prev_action)
		.expect(&expect_action::to_have_child(&ev, ev.prev_action));
	if index == children.len() - 1 {
		ev.trigger_bubble(commands);
	} else {
		ev.trigger_run(commands, children[index + 1], ());
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

		let on_result = observe_trigger_names::<OnResult>(world);
		let on_run = observe_triggers::<OnRun>(world);

		world
			.spawn((Name::new("root"), FallbackFlow))
			.with_children(|parent| {
				parent.spawn((
					Name::new("child1"),
					RespondWith(RunResult::Failure),
				));
				parent.spawn((
					Name::new("child2"),
					RespondWith(RunResult::Success),
				));
			})
			.flush_trigger(OnRun::local());

		expect(&on_run).to_have_been_called_times(3);
		expect(&on_result).to_have_been_called_times(3);
		expect(&on_result).to_have_returned_nth_with(0, &"child1".to_string());
		expect(&on_result).to_have_returned_nth_with(1, &"child2".to_string());
		expect(&on_result).to_have_returned_nth_with(2, &"root".to_string());
	}
}
