use crate::prelude::*;
use bevy::prelude::*;

/// A simple action that will succeed a certain number of times before failing.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect, Action)]
#[observers(succeed_times)]
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
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	mut query: Query<&mut SucceedTimes>,
) {
	let mut action = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	if action.times < action.max_times {
		action.times += 1;
		commands
			.entity(trigger.entity())
			.trigger(OnRunResult::success());
	} else {
		commands
			.entity(trigger.entity())
			.trigger(OnRunResult::failure());
	}
}
