// use beet_flow::action_observers;
use crate::prelude::*;
use bevy::prelude::*;

/// Chain runs together, even if they are not in the same hierarchy,
/// this is useful for a State Machine pattern, but be aware that
/// in terms of control flow this is essentially a [`goto`](https://xkcd.com/292/) statement.
/// 
/// The `origin` will be preserved in calling the next OnRun.
#[action(run_next)]
#[derive(Debug, Component, PartialEq, Eq)]
pub struct RunNext {
	pub action: Entity,
	/// if set, this will only run next if the result matches this,
	/// otherwise it will stop repeating and trigger OnChildResult<RunResult>
	/// on its parent.
	pub if_result_matches: Option<RunResult>,
}

impl RunNext {
	pub fn new(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: None,
		}
	}
	pub fn if_success(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: Some(RunResult::Success),
		}
	}
	pub fn if_failure(action: Entity) -> Self {
		Self {
			action,
			if_result_matches: Some(RunResult::Failure),
		}
	}
}


fn run_next(
	ev: Trigger<OnResult>,
	mut commands: Commands,
	query: Query<&RunNext>,
) {
	let run_next = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	if let Some(check) = &run_next.if_result_matches {
		if &ev.payload != check {
			return;
		}
	}
	commands.trigger(OnRunAction::global_with_origin(
		run_next.action,
		ev.origin,
		(),
	));
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
		let world = app.world_mut();
		let action1 = world.spawn(ReturnWith(RunResult::Success)).id();
		let action2 = world
			.spawn((RunNext::new(action1), ReturnWith(RunResult::Success)))
			.flush_trigger(OnRun::local())
			.id();

		expect(&observed).to_have_been_called_times(2);
		expect(&observed).to_have_returned_nth_with(
			0,
			&OnResultAction::global(action2, RunResult::Success),
		);
		expect(&observed).to_have_returned_nth_with(
			1,
			&OnResultAction::global_with_origin(
				action1,
				action2,
				RunResult::Success,
			),
		);
	}
}
