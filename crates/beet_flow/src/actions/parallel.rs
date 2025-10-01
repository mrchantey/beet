use crate::prelude::*;
use bevy::platform::collections::HashSet;
use beet_core::prelude::*;

/// An action that runs all of its children in parallel.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Logic
/// - If a child fails it will fail immediately.
/// - If all children succeed it will succeed.
/// ## Example
/// Run two children in parallel
/// ```
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world
///		.spawn(Parallel::default())
///		.with_child(ReturnWith(RunResult::Success))
///		.with_child(ReturnWith(RunResult::Success))
///		.trigger(OnRun::local());
/// ```
#[action(on_start, on_next)]
#[derive(Default, Component, Deref, DerefMut, Reflect)]
#[reflect(Default, Component)]
// TODO sparseset
pub struct Parallel(pub HashSet<Entity>);

fn on_start(
	ev: On<Run>,
	mut commands: Commands,
	mut query: Query<(&mut Parallel, &Children)>,
) {
	let (mut action, children) = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_children(&ev));
	action.clear();

	for child in children {
		ev.trigger_next(&mut commands.reborrow(), *child);
	}
}


fn on_next(
	ev: On<OnChildResult>,
	commands: Commands,
	mut query: Query<(&mut Parallel, &Children)>,
) {
	if ev.payload == RunResult::Failure {
		ev.trigger_bubble(commands);
		return;
	}
	let (mut action, children) = query
		.get_mut(ev.parent)
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
	use beet_core::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn fails() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let on_result = observer_ext::observe_triggers::<OnResultAction>(world);
		let on_run = observer_ext::observe_triggers::<OnRun>(world);

		let action = world
			.spawn((Name::new("root"), Parallel::default()))
			.with_child((Name::new("child1"), ReturnWith(RunResult::Success)))
			.with_child((Name::new("child2"), ReturnWith(RunResult::Failure)))
			.flush_trigger(OnRun::local())
			.id();

		on_run.len().xpect_eq(3);
		on_result.len().xpect_eq(3);
		on_result
			.get_index(2)
			.xpect_eq(Some(OnResultAction::global(action, RunResult::Failure)));
	}
	#[test]
	fn succeeds() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let on_result = observer_ext::observe_triggers::<OnResultAction>(world);
		let on_run = observer_ext::observe_triggers::<OnRun>(world);

		let action = world
			.spawn((Name::new("root"), Parallel::default()))
			.with_child((Name::new("child1"), ReturnWith(RunResult::Success)))
			.with_child((Name::new("child2"), ReturnWith(RunResult::Success)))
			.flush_trigger(OnRun::local())
			.id();

		on_run.len().xpect_eq(3);
		on_result.len().xpect_eq(3);
		on_result
			.get_index(2)
			.xpect_eq(Some(OnResultAction::global(action, RunResult::Success)));
	}
}
