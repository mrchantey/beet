use crate::prelude::*;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use std::fmt::Debug;

/// Tracks the last time a node was run.
/// This action is required by [`ContinueRun`] so is rarely added manually.
#[derive(Default, Debug, Component, Action, Reflect)]
#[reflect(Component, Default)]
#[observers(on_start, on_stop)]
#[systems(
	update_run_timers
		.in_set(PreTickSet)
)]
pub struct RunTimer {
	/// Last time the node was last started, or time since level load if never started.
	pub last_started: Stopwatch,
	/// Last time the node was last stopped, or time since level load if never stopped.
	pub last_stopped: Stopwatch,
}


fn on_start(trigger: Trigger<OnAdd, Running>, mut query: Query<&mut RunTimer>) {
	query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING)
		.last_started
		.reset();
}
fn on_stop(
	trigger: Trigger<OnRemove, Running>,
	mut query: Query<&mut RunTimer>,
) {
	query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING)
		.last_stopped
		.reset();
}

/// Syncs [`RunTimer`] components, by default added to [`PreTickSet`].
/// This is added to the [`PreTickSet`], any changes detected were from the previous frame.
/// For this reason timers are reset before they tick to accuratly indicate when the [`Running`]
/// component was *actually* added or removed.
pub fn update_run_timers(
	// TODO run_if
	time: Res<Time>,
	mut timers: Query<&mut RunTimer>,
) {
	for mut timer in timers.iter_mut() {
		timer.last_started.tick(time.delta());
		timer.last_stopped.tick(time.delta());
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	pub fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(LifecyclePlugin);
		app.insert_time();

		let entity = app.world_mut().spawn((Running, RunTimer::default())).id();

		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(entity).unwrap();
		expect(timer.last_started.elapsed_secs()).to_be_close_to(1.0)?;
		expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0)?;

		app.world_mut().entity_mut(entity).remove::<Running>();
		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(entity).unwrap();
		expect(timer.last_started.elapsed_secs()).to_be_close_to(2.0)?;
		expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0)?;

		Ok(())
	}
}
