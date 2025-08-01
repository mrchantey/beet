use crate::prelude::*;
use beet_core::prelude::When;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use std::fmt::Debug;

/// Tracks the last time a node was run.
/// This action is required by [`ContinueRun`] so is rarely added manually.
/// Note that even when not running the timers will still tick, which
/// allows for 'Run if inactive for duration' etc.
/// For an example usage see [`ReturnInDuration`].
#[derive(Default, Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RunTimer {
	/// Last time the node was last started, or time since level load if never started.
	pub last_started: Stopwatch,
	/// Last time the node was last stopped, or time since level load if never stopped.
	pub last_stopped: Stopwatch,
}

pub(crate) fn reset_run_time_started(
	ev: Trigger<OnAdd, Running>,
	mut query: Query<&mut RunTimer>,
) {
	// println!("reset_run_time_started");
	query
		.get_mut(ev.target())
		.map(|mut timer| timer.last_started.reset())
		.ok();
}
pub(crate) fn reset_run_timer_stopped(
	ev: Trigger<OnRemove, Running>,
	mut query: Query<&mut RunTimer>,
) {
	// println!("reset_run_time_stopped");
	query
		.get_mut(ev.target())
		.map(|mut timer| timer.last_stopped.reset())
		.ok();
}

/// Ticks all [`RunTimer`] timers in the [`PreTickSet`].
pub(crate) fn tick_run_timers(
	time: When<Res<Time>>,
	mut timers: Populated<&mut RunTimer>,
) {
	for mut timer in timers.iter_mut() {
		timer.last_started.tick(time.delta());
		timer.last_stopped.tick(time.delta());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	pub fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		app.insert_time();

		let entity = app
			.world_mut()
			.spawn((Running::default(), RunTimer::default()))
			.id();

		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(entity).unwrap();
		expect(timer.last_started.elapsed_secs()).to_be_close_to(1.0);
		expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0);

		app.world_mut().entity_mut(entity).remove::<Running>();
		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(entity).unwrap();
		expect(timer.last_started.elapsed_secs()).to_be_close_to(2.0);
		expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0);
	}
}
