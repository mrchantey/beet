// use beet_flow::action_observers;
use crate::prelude::*;
use bevy::prelude::*;

/// Immediately return a provided value when [`OnRun`] is called,
/// regardless of the world state.
/// As an analogy this is similar to a `const` variable, although
/// it technically can be changed by some external system.
/// ## Tags
/// - [ControlFlow](ActionTag::ControlFlow)
/// ## Example
/// returns `RunResult::Success` when triggered.
/// ```
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world
/// 	.spawn(ReturnWith(RunResult::Success))
/// 	.trigger(OnRun::local());
/// ```
#[action(return_with::<T>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct ReturnWith<T: ResultPayload>(pub T);

fn return_with<T: ResultPayload>(
	ev: Trigger<OnRun<T::Run>>,
	commands: Commands,
	action: Query<&ReturnWith<T>>,
) {
	let payload = action
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev))
		.0
		.clone();
	ev.trigger_result(commands, payload);
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

		let observed = observe_triggers::<OnResultAction>(app.world_mut());
		let entity = app
			.world_mut()
			.spawn(ReturnWith(RunResult::Success))
			.flush_trigger(OnRun::local())
			.id();

		expect(&observed).to_have_been_called_times(1);
		expect(&observed).to_have_returned_nth_with(
			0,
			&OnResultAction::global(entity, RunResult::Success),
		);
	}
}
