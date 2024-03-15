use crate::prelude::*;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use std::fmt::Debug;

/// Tracks the last time a node was run.
#[derive(Default, Debug, Component, Reflect)]
#[reflect(Component)]
pub struct RunTimer {
	/// Last time the node was last started, or time since level load if never started.
	pub last_started: Stopwatch,
	/// Last time the node was last stopped, or time since level load if never stopped.
	pub last_stopped: Stopwatch,
}




/// Syncs [`RunTimer`] components, by default added to [`PreTickSet`].
/// This is added to the [`PreTickSet`], any changes detected were from the previous frame.
/// For this reason timers are reset before they tick to accuratly indicate when the [`Running`]
/// component was *actually* added or removed.
pub fn update_run_timers(
	time: Res<Time>,
	mut timers: Query<&mut RunTimer>,
	added: Query<Entity, Added<Running>>,
	mut removed: RemovedComponents<Running>,
) {
	// 1. reset timers

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

	// 2. tick timers

	for mut timer in timers.iter_mut() {
		timer.last_started.tick(time.delta());
		timer.last_stopped.tick(time.delta());
	}
}
