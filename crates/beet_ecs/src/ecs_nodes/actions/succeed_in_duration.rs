use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_utils::Duration;
use serde::Deserialize;
use serde::Serialize;

#[action(system=succeed_in_duration)]
pub struct SucceedInDuration {
	pub duration: Duration,
}

impl Default for SucceedInDuration {
	fn default() -> Self {
		Self {
			duration: Duration::from_secs(1),
		}
	}
}

pub fn succeed_in_duration(
	mut commands: Commands,
	mut query: Query<(Entity, &RunTimer, &SucceedInDuration), With<Running>>,
) {
	for (entity, timer, succeed_in_duration) in query.iter_mut() {
		if timer.last_started.elapsed() >= succeed_in_duration.duration {
			commands.entity(entity).insert(RunResult::Success);
		}
	}
}
