use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashSet;


/// An action that runs all of its children in parallel.
/// - All results will bubble up, so expect multiple [`OnResult`] triggers
#[action(on_start, on_next)]
#[derive(Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
pub struct ParallelFlow(pub HashSet<Entity>);

fn on_start(
	ev: Trigger<OnRun>,
	mut commands: Commands,
	mut query: Query<(&mut ParallelFlow, &Children)>,
) {
	let (mut action, children) = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_children(&ev));
	action.clear();

	for child in children {
		ev.trigger_next(commands.reborrow(), *child);
	}
}


fn on_next(
	ev: Trigger<OnChildResult>,
	commands: Commands,
	mut query: Query<(&mut ParallelFlow, &Children)>,
) {
	if ev.payload == RunResult::Failure {
		ev.trigger_bubble(commands);
		return;
	}
	let (mut action, children) = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	action.insert(ev.child);

	if action.len() == children.iter().len() {
		ev.trigger_bubble(commands);
		return;
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn fails() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let on_result = observe_triggers::<OnResultAction>(world);
		let on_run = observe_triggers::<OnRun>(world);

		let action = world
			.spawn((Name::new("root"), ParallelFlow::default()))
			.with_child((Name::new("child1"), RespondWith(RunResult::Success)))
			.with_child((Name::new("child2"), RespondWith(RunResult::Failure)))
			.flush_trigger(OnRun::local())
			.id();

		expect(&on_run).to_have_been_called_times(3);
		expect(&on_result).to_have_been_called_times(3);
		expect(&on_result).to_have_returned_nth_with(
			2,
			&OnResultAction::global(action, RunResult::Failure),
		);
	}
	#[test]
	fn succeeds() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let on_result = observe_triggers::<OnResultAction>(world);
		let on_run = observe_triggers::<OnRun>(world);

		let action = world
			.spawn((Name::new("root"), ParallelFlow::default()))
			.with_child((Name::new("child1"), RespondWith(RunResult::Success)))
			.with_child((Name::new("child2"), RespondWith(RunResult::Success)))
			.flush_trigger(OnRun::local())
			.id();

		expect(&on_run).to_have_been_called_times(3);
		expect(&on_result).to_have_been_called_times(3);
		expect(&on_result).to_have_returned_nth_with(
			2,
			&OnResultAction::global(action, RunResult::Success),
		);
	}
}
