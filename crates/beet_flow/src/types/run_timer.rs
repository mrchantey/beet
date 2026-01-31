//! Timer tracking for action execution duration.
use crate::prelude::*;
use beet_core::prelude::*;
use std::fmt::Debug;

/// Tracks elapsed time since last run and end events for an action.
///
/// This component is automatically added by [`ContinueRun`] and tracks two
/// independent timers:
/// - `last_run`: Time since the action was last started, ie the last [`GetOutcome`] event.
/// - `last_end`: Time since the action last completed, ie the last [`Outcome`] event.
///
/// Both timers tick continuously, even when the action is not running. This
/// allows patterns like "run if inactive for duration" to be implemented.
///
/// # Example
///
/// For duration-based endings, see [`EndInDuration`]:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// world.spawn((
///     Running,
///     EndInDuration::pass(Duration::from_secs(2)),
/// ));
/// ```
#[derive(Default, Debug, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RunTimer {
	/// Time since the action was last started, reset when [`Running`] is added.
	pub last_run: Stopwatch,
	/// Time since the action last completed, reset when [`Running`] is removed.
	pub last_end: Stopwatch,
}

/// Ticks all [`RunTimer`] components in the [`PreTickSet`].
pub(crate) fn tick_run_timers(
	time: When<Res<Time>>,
	mut timers: Populated<&mut RunTimer>,
) {
	for mut timer in timers.iter_mut() {
		timer.last_run.tick(time.delta());
		timer.last_end.tick(time.delta());
	}
}

/// Resets `last_run` when [`Running`] is added.
pub(crate) fn reset_run_time_started(
	ev: On<Add, Running>,
	mut query: Populated<&mut RunTimer>,
) -> Result {
	query.get_mut(ev.event().event_target())?.last_run.reset();
	Ok(())
}

/// Resets `last_end` when [`Running`] is removed.
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
