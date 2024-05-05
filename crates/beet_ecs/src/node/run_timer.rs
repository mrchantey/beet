use crate::prelude::*;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use std::fmt::Debug;

/// Tracks the last time a node was run.
#[derive(Default, Debug, Component, Reflect)]
#[reflect(Component, Default)]
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

		let root = EmptyAction.into_beet_builder().build(app.world_mut()).value;

		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(root).unwrap();
		expect(timer.last_started.elapsed_secs()).to_be_close_to(1.0)?;
		expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0)?;

		app.world_mut().entity_mut(root).remove::<Running>();
		app.update_with_secs(1);

		let timer = app.world().get::<RunTimer>(root).unwrap();
		expect(timer.last_started.elapsed_secs()).to_be_close_to(2.0)?;
		expect(timer.last_stopped.elapsed_secs()).to_be_close_to(1.0)?;

		Ok(())
	}
}
