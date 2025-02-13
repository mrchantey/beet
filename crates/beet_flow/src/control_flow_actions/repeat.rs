use crate::prelude::*;
use bevy::prelude::*;




/// Reattaches the [`RunOnSpawn`] component whenever [`OnResult`] is called.
/// Using [`RunOnSpawn`] means this does **not** directly trigger observers, which avoids infinite loops.
///
/// Note that [`RepeatFlow`] requires [`NoBubble`] so results must be bubbled up manually.
/// ```
/// # use bevy::prelude::*;
/// # use beet_flow::prelude::*;
/// // this example will repeat the action twice, then bubble up the failure
/// World::new()
/// .spawn((Repeat::if_success(), SucceedTimes::new(2)))
/// .trigger(OnRun::local());
/// ```
#[action(repeat)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(NoBubble)]
pub struct Repeat {
	/// Optional predicate to only repeat if the result matches.
	pub if_result_matches: Option<RunResult>,
}

impl Repeat {
	/// Repeats the action if the result is [`RunResult::Success`].
	pub fn if_success() -> Self {
		Self {
			if_result_matches: Some(RunResult::Success),
		}
	}
	/// Repeats the action if the result is [`RunResult::Failure`].
	pub fn if_failure() -> Self {
		Self {
			if_result_matches: Some(RunResult::Failure),
		}
	}
}

impl Default for Repeat {
	fn default() -> Self {
		Self {
			if_result_matches: None,
		}
	}
}

fn repeat(
	ev: Trigger<OnResult>,
	parents: Query<&Parent>,
	action_observers: Query<&ActionObservers>,
	query: Query<&Repeat>,
	mut commands: Commands,
) {
	let repeat = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	if let Some(check) = &repeat.if_result_matches {
		if &ev.payload != check {
			// repeat is completed, call OnResult
			OnChildResult::try_trigger(
				commands,
				parents,
				action_observers,
				ev.action,
				ev.origin,
				ev.payload.clone(),
			);
			return;
		}
	}
	// otherwise run again on the next tick
	let action = OnRunAction::new(ev.action, ev.origin, ());
	commands.entity(ev.action).insert(RunOnSpawn::new(action));
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn repeat_always() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();
		let func = observe_triggers::<OnResultAction>(world);

		world
			.spawn((Repeat::default(), SucceedTimes::new(2)))
			.flush_trigger(OnRun::local());

		expect(&func).to_have_been_called_times(1);
		app.update();
		expect(&func).to_have_been_called_times(2);
		app.update();
		expect(&func).to_have_been_called_times(3);
		app.update();
		// // even though child failed, it keeps repeating
		expect(&func).to_have_been_called_times(4);
		app.update();
		expect(&func).to_have_been_called_times(5);
	}

	#[test]
	fn repeat_if() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();
		let func = observe_triggers::<OnResultAction>(world);

		world
			.spawn((Repeat::if_success(), SucceedTimes::new(2)))
			.flush_trigger(OnRun::local());

		expect(&func).to_have_been_called_times(1);
		app.update();
		expect(&func).to_have_been_called_times(2);
		app.update();
		expect(&func).to_have_been_called_times(3);
		app.update();
		// it stopped repeating
		expect(&func).to_have_been_called_times(3);
		app.update();
		expect(&func).to_have_been_called_times(3);
	}
	#[test]
	fn repeat_child() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();
		let func = observe_triggers::<OnResultAction>(world);

		world
			.spawn((Sequence, Repeat::if_success()))
			.with_child(SucceedTimes::new(2))
			.flush_trigger(OnRun::local());

		expect(&func).to_have_been_called_times(2);
		app.update();
		expect(&func).to_have_been_called_times(4);
		app.update();
		expect(&func).to_have_been_called_times(6);
		app.update();
		expect(&func).to_have_been_called_times(6);
		app.update();
		// last one, it stopped repeating
		expect(&func).to_have_been_called_times(6);
	}
}
