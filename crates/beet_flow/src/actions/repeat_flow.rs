use crate::prelude::*;
use bevy::prelude::*;




/// Reattaches the [`RunOnSpawn`] component whenever [`OnResult`] is called.
/// Using [`RunOnSpawn`] means this does **not** directly trigger observers, which avoids infinite loops.
///
/// Note that [RepeatFlow] requires [NoBubble] so results must be bubbled up manually.
#[action(repeat)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(NoBubble)]
pub struct RepeatFlow {
	pub if_result_matches: Option<RunResult>,
}

impl RepeatFlow {
	pub fn if_success() -> Self {
		Self {
			if_result_matches: Some(RunResult::Success),
		}
	}
	pub fn if_failure() -> Self {
		Self {
			if_result_matches: Some(RunResult::Failure),
		}
	}
}

impl Default for RepeatFlow {
	fn default() -> Self {
		Self {
			if_result_matches: None,
		}
	}
}

fn repeat(
	ev: Trigger<OnChildResult>,
	// parents: Query<&Parent>,
	query: Query<&RepeatFlow>,
	mut commands: Commands,
) {
	let action = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	if let Some(check) = &action.if_result_matches {
		if &ev.payload != check {
			// repeat is completed, call OnResult
			ev.trigger_bubble(commands);
			return;
		}
	}
	commands.entity(ev.action).insert(RunOnSpawn::default());
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
		let func = observe_triggers::<OnResult>(world);

		world
			.spawn((SequenceFlow, RepeatFlow::default()))
			.with_child(SucceedTimes::new(2))
			.flush_trigger(OnRun::local());

		expect(&func).to_have_been_called_times(2);
		app.update();
		expect(&func).to_have_been_called_times(4);
		app.update();
		expect(&func).to_have_been_called_times(6);
		app.update();
		// even though child failed, it keeps repeating
		expect(&func).to_have_been_called_times(8);
	}

	#[test]
	fn repeat_if() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();
		let func = observe_triggers::<OnResult>(world);

		world
			.spawn((SequenceFlow, RepeatFlow::if_success()))
			.with_child(SucceedTimes::new(2))
			.flush_trigger(OnRun::local());

		expect(&func).to_have_been_called_times(2);
		app.update();
		expect(&func).to_have_been_called_times(4);
		app.update();
		expect(&func).to_have_been_called_times(7);
		app.update();
		expect(&func).to_have_been_called_times(7);
		app.update();
		// last one, it stopped repeating
		expect(&func).to_have_been_called_times(7);
	}
}
