use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_time::Stopwatch;
use bevy_time::Time;
use std::fmt::Debug;

/// Tracks the last time a node was run.
#[derive(Default, Debug, Component)]
pub struct RunTimer {
	/// Last time the node was last started, or time since level load if never started.
	pub last_started: Stopwatch,
	/// Last time the node was last stopped, or time since level load if never stopped.
	pub last_stopped: Stopwatch,
}




/// Syncs [`RunTimer`] components, by default added to [`PreNodeUpdateSet`].
pub fn sync_run_timers(
	time: Res<Time>,
	mut timers: Query<&mut RunTimer>,
	added: Query<Entity, Added<Running>>,
	mut removed: RemovedComponents<Running>,
) {
	for mut timer in timers.iter_mut() {
		timer.last_started.tick(time.delta());
		timer.last_stopped.tick(time.delta());
	}

	for added in added.iter() {
		if let Ok(mut timer) = timers.get_mut(added) {
			timer.last_started.reset();
		}
	}

	for removed in removed.read() {
		if let Ok(mut timer) = timers.get_mut(removed) {
			timer.last_stopped.reset();
		}
	}
}
