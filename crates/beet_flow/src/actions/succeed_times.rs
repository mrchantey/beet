use crate::prelude::*;
use bevy::prelude::*;

/// A simple action that will succeed a certain number of times before failing.
#[action(succeed_times)]
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct SucceedTimes {
	pub times: usize,
	pub max_times: usize,
}

impl SucceedTimes {
	pub fn new(max_times: usize) -> Self {
		Self {
			times: 0,
			max_times,
		}
	}
	pub fn reset(&mut self) { self.times = 0; }
}

fn succeed_times(
	ev: Trigger<OnRun>,
	commands: Commands,
	mut query: Query<&mut SucceedTimes>,
) {
	let mut action = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_action(&ev));

	if action.times < action.max_times {
		action.times += 1;
		ev.trigger_result(commands, RunResult::Success);
	} else {
		ev.trigger_result(commands, RunResult::Failure);
	}
}



// tested with RepeatFlow
