use crate::prelude::*;
use bevy::prelude::*;




/// Reattaches the [`RunOnSpawn`] component whenever [`OnRunResult`] is called.
/// Using [`RunOnSpawn`] means this does **not** directly trigger observers, which avoids infinite loops.
///
/// Note that [RepeatFlow] requires [NoBubble] so results must be bubbled up manually.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(repeat)]
#[require(NoBubble)]
pub struct RepeatFlow {
	// TODO times
	// pub times: RepeatAnimation,
	/// if set, this will only repeat if the result matches this,
	/// otherwise it will stop repeating and trigger OnChildResult<RunResult>
	/// on its parent.
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
			// times: RepeatAnimation::Forever,
		}
	}
}

fn repeat(
	trigger: Trigger<OnRunResult>,
	parents: Query<&Parent>,
	query: Query<&RepeatFlow>,
	mut commands: Commands,
) {
	let flow = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);
	if let Some(check) = flow.if_result_matches {
		let result = trigger.event().result();
		if result != check {
			// repeat is completed, try to bubble up the result
			if let Ok(parent) = parents.get(trigger.entity()) {
				commands
					.entity(parent.get())
					.trigger(OnChildResult::new(trigger.entity(), result));
			}
			return;
		}
	}

	// println!("repeat for {}", name_or_entity(&names, trigger.entity()));
	commands.entity(trigger.entity()).insert(RunOnSpawn);
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;
	use world_ext::EntityWorldMutwExt;


	fn init() -> App {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<(
			SequenceFlow,
			SucceedTimes,
			RepeatFlow,
			RunOnSpawn,
		)>::default());
		let world = app.world_mut();
		world.add_observer(bubble_run_result);

		app
	}

	#[test]
	fn repeat_always() {
		let mut app = init();
		let world = app.world_mut();
		let func = observe_triggers::<OnRunResult>(world);

		world
			.spawn((SequenceFlow, RepeatFlow::default()))
			.with_child(SucceedTimes::new(2))
			.flush_trigger(OnRun);

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
		let mut app = init();
		let world = app.world_mut();
		let func = observe_triggers::<OnRunResult>(world);

		world
			.spawn((SequenceFlow, RepeatFlow::if_success()))
			.with_child(SucceedTimes::new(2))
			.flush_trigger(OnRun);

		expect(&func).to_have_been_called_times(2);
		app.update();
		expect(&func).to_have_been_called_times(4);
		app.update();
		expect(&func).to_have_been_called_times(6);
		app.update();
		// last one, it stopped repeating
		expect(&func).to_have_been_called_times(6);
	}
}
