use crate::prelude::*;
use bevy::prelude::*;

/// An action that runs all of its children in order until one fails.
/// - If a child succeeds it will run the next child.
/// - If there are no more children to run it will succeed.
/// - If a child fails it will fail.
#[action(on_start, on_next)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Sequence;

fn on_start(ev: Trigger<OnRun>, commands: Commands, query: Query<&Children>) {
	let children = query
		.get(ev.action)
		.expect(&expect_action::to_have_children(&ev));
	if let Some(first_child) = children.iter().next() {
		ev.trigger_next(commands, *first_child);
	} else {
		ev.trigger_result(commands, RunResult::Success);
	}
}

fn on_next(
	ev: Trigger<OnChildResult>,
	commands: Commands,
	query: Query<&Children>,
) {
	if ev.payload == RunResult::Failure {
		ev.trigger_bubble(commands);
		return;
	}
	let children = query
		.get(ev.parent)
		.expect(&expect_action::to_have_children(&ev));
	let index = children
		.iter()
		.position(|&x| x == ev.child)
		.expect(&expect_action::to_have_child(&ev, ev.child));
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

	#[rustfmt::skip]
	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let on_result = collect_on_result(world);
		let on_run = collect_on_run(world);

		world
			.spawn((Name::new("root"), Sequence))
			.with_child((Name::new("child1"), ReturnWith(RunResult::Success)))
			.with_child((Name::new("child2"), ReturnWith(RunResult::Success)))
			.flush_trigger(OnRun::local());

		expect(on_run()).to_be(vec![
			"root".to_string(),
			"child1".to_string(),
			"child2".to_string(),
		]);
		expect(on_result()).to_be(vec![
			("child1".to_string(), RunResult::Success),
			("child2".to_string(), RunResult::Success),
			("root".to_string(), RunResult::Success),
		]);
	}
}
