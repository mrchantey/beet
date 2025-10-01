use crate::prelude::*;
use beet_core::prelude::*;

/// Succeed a certain number of times before failing.
/// ## Tags
/// - [`ControlFlow`](ActionTag::ControlFlow)
/// - [`LongRunning`](ActionTag::LongRunning)
///
/// For example usage see [`Repeat`].
#[action(succeed_times)]
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct SucceedTimes {
	/// The number of times the action has executed.
	pub times: usize,
	/// The number of times the action should succeed before failing.
	pub max_times: usize,
}

impl SucceedTimes {
	/// Specify the number of times the action should succeed before failing.
	pub fn new(max_times: usize) -> Self {
		Self {
			times: 0,
			max_times,
		}
	}
	/// Reset the number of times the action has executed.
	pub fn reset(&mut self) { self.times = 0; }
}

fn succeed_times(
	ev: On<Run>,
	mut commands: Commands,
	mut query: Query<&mut SucceedTimes>,
) {
	let mut action = query
		.get_mut(ev.action)
		.expect(&expect_action::to_have_action(&ev));

	if action.times < action.max_times {
		action.times += 1;
		ev.trigger_result(&mut commands, RunResult::Success);
	} else {
		ev.trigger_result(&mut commands, RunResult::Failure);
	}
}


// tested with [Repeat]
