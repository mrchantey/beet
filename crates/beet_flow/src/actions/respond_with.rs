// use beet_flow::action_observers;
use crate::prelude::*;
use bevy::prelude::*;

/// Immediately respond to a given request with a response,
/// no matter the state of the world or the content of the request.
#[action(respond_with::<T>)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct RespondWith<T: ResultPayload>(pub T);

fn respond_with<T: ResultPayload>(
	trig: Trigger<OnRun<T::Run>>,
	commands: Commands,
	action: Query<&RespondWith<T>>,
) {
	let payload = action
		.get(trig.action)
		.expect(&expect_action::to_have_action(&trig))
		.0
		.clone();
	trig.trigger_result(commands, payload);
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

		let observed = observe_triggers::<OnResult>(app.world_mut());
		let entity = app
			.world_mut()
			.spawn(RespondWith(RunResult::Success))
			.flush_trigger(OnRun::local())
			.id();

		expect(&observed).to_have_been_called_times(1);
		expect(&observed).to_have_returned_nth_with(
			0,
			&OnResult::new_global(entity, RunResult::Success),
		);
	}
}
