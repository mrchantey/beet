use crate::prelude::*;
use beet_core::prelude::*;
use std::fmt::Debug;

/// Tracks the last time a `Run` and `End` was triggered on this entity.
/// This action is required by [`ContinueRun`] so is rarely added manually.
/// Note that even when not running the timers will still tick, which
/// allows for 'Run if inactive for duration' etc.
/// For an example usage see [`EndInDuration`].
#[derive(Default, Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RunTimer {
	/// Last time the node was last started, or time since level load if never started.
	pub last_run: Stopwatch,
	/// Last time the node was last stopped, or time since level load if never stopped.
	pub last_end: Stopwatch,
}

/// Ticks all [`RunTimer`] timers in the [`PreTickSet`].
pub(crate) fn tick_run_timers(
	time: When<Res<Time>>,
	mut timers: Populated<&mut RunTimer>,
) {
	for mut timer in timers.iter_mut() {
		timer.last_run.tick(time.delta());
		timer.last_end.tick(time.delta());
	}
}

pub(crate) fn reset_run_time_started(
	ev: On<Add, Running>,
	mut query: Populated<&mut RunTimer>,
) -> Result {
	query.get_mut(ev.event().event_target())?.last_run.reset();
	Ok(())
}
pub(crate) fn reset_run_timer_stopped(
	ev: On<Remove, Running>,
	mut query: Populated<&mut RunTimer>,
) -> Result {
	query.get_mut(ev.event().event_target())?.last_end.reset();
	Ok(())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	pub fn works() {
		let mut app = App::new();
		app.add_plugins(ControlFlowPlugin::default());
		app.insert_time();

		let entity = app.world_mut().spawn(Running).id();

		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(entity).unwrap();
		timer.last_run.elapsed_secs().xpect_close(1.0);
		timer.last_end.elapsed_secs().xpect_close(1.0);

		app.world_mut().entity_mut(entity).remove::<Running>();
		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(entity).unwrap();
		timer.last_run.elapsed_secs().xpect_close(2.0);
		timer.last_end.elapsed_secs().xpect_close(1.0);
	}
}
