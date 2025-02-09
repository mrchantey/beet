use crate::prelude::*;
use bevy::prelude::*;
/// Immediately end the run with the provided result
#[derive(Debug, GlobalAction, Clone, Copy, PartialEq, Deref, DerefMut)]
#[observers(end_on_run)]
pub struct EndOnRunGlobal(pub RunResult);

fn end_on_run(
	trigger: Trigger<OnAction>,
	actions: Query<&EndOnRunGlobal>,
	commands: Commands,
) {
	let action = actions
		.get(trigger.action)
		.expect(expect_action::ACTION_QUERY_MISSING);
	trigger.on_result(commands, **action);
}

impl EndOnRunGlobal {
	pub fn new(result: RunResult) -> Self { Self(result) }
	pub fn success() -> Self { Self(RunResult::Success) }
	pub fn failure() -> Self { Self(RunResult::Failure) }
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
		let func = observe_triggers::<OnRunResultGlobal>(world);
		let entity = world
			.spawn(EndOnRunGlobal::failure())
			.flush_trigger(OnRunGlobal::new())
			.id();
		expect(&func).to_have_been_called();
		expect(&func)
			.to_have_returned_nth_with(0, &OnRunResultGlobal::failure(entity));
	}
}
